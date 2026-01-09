# pg-app-role-manager

CLI tool for managing PostgreSQL schema ownership patterns with automatic ownership transfer via event triggers.

## Overview

This tool implements a pattern where:
1. Schemas are mapped to specific PostgreSQL roles
2. Event triggers automatically transfer ownership of new objects to the mapped role
3. Schema-to-role mappings are **immutable** after initialization

## Build

```bash
# Static binary (recommended - portable across Linux distributions)
cargo build --release --target x86_64-unknown-linux-musl
```

Binary location: `target/x86_64-unknown-linux-musl/release/pg-app-role-manager`

## Commands

### init - Initialize Schema Ownership Pattern

Creates database (if needed), schema, role, config table, and event trigger. Once initialized, the schema-to-role mapping is immutable.

```bash
pg-app-role-manager init --database mydb --schema app --role app_owner
```

Creates:
- Database (if it doesn't exist)
- Schema and role (with NOLOGIN)
- Config table in `public.schema_ownership_config`
- Event trigger function for automatic ownership transfer
- Event trigger `auto_transfer_schema_ownership_trigger`
- Initial schema-to-role mapping

**System databases blocked:** postgres, template0, template1, rdsadmin, azure_maintenance, cloudsqladmin

### list-mappings - View All Schema-to-Role Mappings

Scans all non-system databases in the PostgreSQL instance and displays schema ownership configuration.

```bash
pg-app-role-manager list-mappings
```

Output includes:
- Database name
- Schema name
- Target role
- Created/updated timestamps
- Summary: total mappings across all databases

**No --dbname required** - automatically scans all user databases.

## Connection Options

Provide connection details via flags or environment variables:

```bash
# Via flags
pg-app-role-manager --host db.example.com --port 5432 --user admin --password secret init ...

# Via environment variables
export PGHOST=db.example.com
export PGPORT=5432
export PGUSER=admin
export PGPASSWORD=secret
pg-app-role-manager init ...
```

**Available options:**
- `--host` / `PGHOST` (default: localhost)
- `--port` / `PGPORT` (default: 5432)
- `--user` / `PGUSER` (required)
- `--password` / `PGPASSWORD` (required, hidden in help output)
- `--dbname` / `PGDATABASE` (optional, used by init if --database not specified)
- `--sslmode` / `PGSSLMODE` (default: prefer)

## TLS/SSL Support

Three SSL modes matching PostgreSQL semantics:

- `--sslmode disable` - No encryption (not recommended for production)
- `--sslmode prefer` (default) - Try TLS first, fallback to unencrypted if TLS fails
- `--sslmode require` - Require TLS encryption (no certificate verification)

**Note:** Certificate verification (verify-ca, verify-full) not implemented. The `require` mode provides encryption but does not verify server identity.

```bash
# Require TLS for production
pg-app-role-manager --sslmode require init --database proddb --schema app --role app_manager

# Disable SSL for local development
pg-app-role-manager --sslmode disable init --database devdb --schema test --role test_role
```

## Verbosity

Control SQL statement logging:

- `-v`: Show SQL statements (excludes trigger function definition)
- `-vv`: Show all SQL including trigger function

```bash
pg-app-role-manager -vv list-mappings
```

## Design Decisions

**Immutable Mappings:** Once a schema is initialized with `init`, its role mapping cannot be changed. This avoids complex ownership transfer logic and potential for inconsistent state.

**Per-Database Config:** The `schema_ownership_config` table is created in each database's `public` schema, not globally. Event triggers are also per-database.

**Idempotent Operations:** Running `init` multiple times is safe - existing objects are skipped.

**No User Grants:** The tool does NOT execute `GRANT role TO user` commands. DBAs must handle user-to-role assignments manually.

## Example Workflow

```bash
# Initialize schema ownership for an application database
pg-app-role-manager init \
  --database myapp_prod \
  --schema app \
  --role app_manager

# View all schema-to-role mappings across the instance
pg-app-role-manager list-mappings

# Create a table in the managed schema (automatic ownership transfer)
psql -d myapp_prod -c "CREATE TABLE app.users (id serial primary key);"
psql -d myapp_prod -c "SELECT tableowner FROM pg_tables WHERE schemaname='app' AND tablename='users';"
# Output: app_manager

# Grant the role to application users (manual step)
psql -d myapp_prod -c "GRANT app_manager TO app_user;"
```

## Security Notes

- Passwords are hidden in `--help` output
- Use TLS (`--sslmode require`) for production deployments
- Store credentials in environment variables, not command arguments
- System databases automatically blocked from management
