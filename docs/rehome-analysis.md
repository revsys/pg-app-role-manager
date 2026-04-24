# rehome Command — Design Analysis

## Problem Statement

PostgreSQL databases commonly accumulate tables, views, functions, and other objects in
the `public` schema before any schema ownership pattern is established. Once
`pg-app-role-manager init` has been run, new objects created in the managed schema
automatically get transferred to the correct owner via the event trigger. But pre-existing
objects in `public` are left behind.

The `rehome` command solves this by moving all non-program objects from `public` to a
caller-specified target schema in a single operation.

---

## What This Program Creates (Must Not Be Moved)

The `init` command creates exactly three objects in `public`:

| Object Kind | Name |
|---|---|
| table | `schema_ownership_config` |
| function | `auto_transfer_schema_ownership()` |
| event trigger | `auto_transfer_schema_ownership_trigger` |

Event triggers are database-level objects and have no schema; they cannot be moved with
`SET SCHEMA`. The table and function must be excluded explicitly.

---

## Object Types and Move Strategy

PostgreSQL's `ALTER ... SET SCHEMA` is the mechanism for all moves. Each object type
requires a different ALTER variant.

### Handled Object Types

| Kind | ALTER Syntax | Notes |
|---|---|---|
| enum | `ALTER TYPE ... SET SCHEMA` | Move before domains that may use them |
| range | `ALTER TYPE ... SET SCHEMA` | Move before domains |
| composite type | `ALTER TYPE ... SET SCHEMA` | Only non-table-backed (`typrelid = 0`) |
| domain | `ALTER TYPE ... SET SCHEMA` | Move after base types they depend on |
| table | `ALTER TABLE ... SET SCHEMA` | Column-owned sequences auto-follow |
| sequence (standalone) | `ALTER SEQUENCE ... SET SCHEMA` | Skip those with `deptype IN ('a','i')` |
| view | `ALTER VIEW ... SET SCHEMA` | Move after tables they depend on |
| materialized view | `ALTER MATERIALIZED VIEW ... SET SCHEMA` | Move after tables/views |
| function | `ALTER FUNCTION ... SET SCHEMA` | Identified by name + arg signature |
| procedure | `ALTER PROCEDURE ... SET SCHEMA` | Distinguished by `prokind = 'p'` |

### Intentionally Ignored

- **Indexes** — belong to tables; they do not have an independent schema and are moved
  automatically when their table moves.
- **Constraints** — belong to tables; same as indexes.
- **DML triggers** — belong to tables; same as indexes.
- **Event triggers** — database-level; no schema applies.
- **Extensions** — require `ALTER EXTENSION ... SET SCHEMA`, but extension objects are
  typically created in the extension's assigned schema, not public. Moving extensions is
  outside scope.
- **Table-backed composite types** — auto-created when a table is defined (`typrelid != 0`);
  they follow the table automatically.
- **Column-owned sequences** — created by `SERIAL`/`BIGSERIAL`/`IDENTITY`; they follow
  their table automatically. Detected via `pg_depend.deptype IN ('a', 'i')`.

---

## Dependency-Safe Move Order

Objects are moved in this order to avoid dependency failures:

1. **Enums** — no intra-schema dependencies
2. **Range types** — no intra-schema dependencies
3. **Composite types** (non-table-backed) — may depend on other types, but composites
   referencing public types are unusual
4. **Domains** — may have CHECK constraints referencing functions; moved after enums/ranges
   that they might reference
5. **Tables** — column-owned sequences follow automatically
6. **Standalone sequences** — after tables (avoids double-move confusion)
7. **Views** — after tables they SELECT from
8. **Materialized views** — after tables and views
9. **Functions/Procedures** — after types they use in signatures; PostgreSQL function
   signatures reference type OIDs so there is no schema-prefix issue at move time

This order handles the common case. Complex circular dependencies (e.g., a domain with a
CHECK constraint calling a function that uses the domain) are rare and will produce a clear
PostgreSQL error message.

---

## Identification Queries

### Exclude program objects

The program's table is excluded by name: `tablename != 'schema_ownership_config'`

The program's function is excluded by name:
`p.proname != 'auto_transfer_schema_ownership'`

### Standalone sequences (not column-owned)

```sql
SELECT c.relname
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
WHERE n.nspname = 'public'
  AND c.relkind = 'S'
  AND NOT EXISTS (
    SELECT 1 FROM pg_depend d
    WHERE d.objid = c.oid AND d.deptype IN ('a', 'i')
  )
```

`deptype = 'a'` is set by `SERIAL`/`BIGSERIAL`; `deptype = 'i'` is used by `IDENTITY`
columns. Both indicate the sequence is owned by a column.

### Non-table-backed composite types

```sql
WHERE t.typtype = 'c' AND t.typrelid = 0
```

`typrelid` is the OID of the backing relation. Zero means it was created with
`CREATE TYPE foo AS (...)` rather than being the implicit row type of a table.

### Functions vs procedures

```sql
SELECT p.proname, pg_get_function_identity_arguments(p.oid) AS args, p.prokind
FROM pg_proc p ...
```

`prokind = 'f'` → function (`ALTER FUNCTION`)
`prokind = 'p'` → procedure (`ALTER PROCEDURE`)

---

## Behavior Decisions

**Requires existing target schema** — `rehome` does not create the schema. The target
schema should be set up with `init` first so that the ownership event trigger will
auto-assign newly moved objects. If the schema does not exist, `rehome` bails with
a helpful message.

**Fail-fast** — consistent with `init` behavior. If any single ALTER fails (e.g., a
cross-schema dependency involving an object outside `public`), the command exits with an
error. Objects moved before the failure remain in the target schema.

**No dry-run** — not in scope for initial implementation; follows the existing command set.

**Idempotency** — if an object has already been moved (it no longer exists in `public`),
the query simply returns no rows for it and nothing is attempted.

---

## File Changes

| File | Change |
|---|---|
| `src/cli.rs` | Add `Rehome { database, schema }` to `Command` enum |
| `src/commands/mod.rs` | Add `pub mod rehome;` |
| `src/main.rs` | Add match arm for `Command::Rehome` |
| `src/report.rs` | Add `ActionOutcome::Moved` variant |
| `src/commands/rehome.rs` | New file — full command implementation |
| `docs/rehome-analysis.md` | This document |
