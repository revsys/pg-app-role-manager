# Rust CLI for PostgreSQL Schema Ownership Pattern - Remaining Tasks

## Project Decisions
- **Binary name**: `pg-app-role-manager`
- **Location**: Current directory (user-config/)
- **Build target**: x86_64-unknown-linux-musl (statically linked)
- **Dependencies**: All pure Rust (no external C libraries required)
- **Scope**: Per-database (config table and triggers in each database, not global)
- **Idempotency**: Skip and continue if objects exist
- **User grants**: NOT implemented (admins handle `GRANT role TO user` manually)
- **TLS semantics**: Matches PostgreSQL (require = encryption without cert verification)
- **Commands**: init, list-mappings only (add-mapping and remove-mapping removed to avoid complexity)
- **Schema owner immutability**: Once initialized, schema-to-role mappings are immutable

---

## Pending Work

### Rehome: Configurable Source Schema — manual DB verification outstanding

Code changes complete and archived (see ARCHIVE.md, "Rehome: Configurable
Source Schema"). `cargo build` (both default and
`x86_64-unknown-linux-musl` targets) and `cargo test` (83 passed) are clean.
Not yet verified against a real/local Postgres instance — no test harness
exists in this repo for that, so it needs a manual pass:

- [ ] `rehome --source-schema legacy_public --target-schema app_schema`
      moves all object types (types, tables, sequences, views, matviews,
      functions/procedures) correctly out of a non-`public` source schema
- [ ] Passing the same name for `--source-schema` and `--target-schema` is
      rejected with the "nothing to rehome" error
- [ ] A nonexistent `--source-schema` is rejected with a clear error
- [ ] `-v` output shows the parameterized query and bound source schema
- [ ] After a rehome from a non-`public` source, `public.schema_ownership_config`
      and `public.auto_transfer_schema_ownership` remain untouched in `public`

---

### Future Enhancements (Not in Current Scope)
- [ ] Dry-run mode (--dry-run flag)
- [ ] Colored output
- [ ] Interactive password prompts
- [ ] Unit and integration tests
- [ ] Terminal width detection for truly dynamic pagination
- [ ] Advanced TLS features (client certificates, custom CA bundles, verify-ca/verify-full modes)

---

See ARCHIVE.md for all completed tasks.
