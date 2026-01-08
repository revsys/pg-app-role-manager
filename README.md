# pg-app-role-manager

CLI tool for managing PostgreSQL schema ownership patterns with automatic ownership transfer via event triggers.

## Build

```bash
# Standard build
cargo build --release

# Static binary (no libc)
cargo build --release --target x86_64-unknown-linux-musl
```

Binary location: `target/release/pg-app-role-manager` or `target/x86_64-unknown-linux-musl/release/pg-app-role-manager`

## Usage

### Initialize Pattern

```bash
pg-app-role-manager init --database mydb --schema app --role app_owner
```

Creates:
- Database (if needed)
- Schema and role
- Config table in `public.schema_ownership_config`
- Event trigger for automatic ownership transfer

### Manage Mappings

```bash
# Add schema-to-role mapping
pg-app-role-manager add-mapping --schema myschema --role myrole

# List all mappings
pg-app-role-manager list-mappings

# Remove mapping
pg-app-role-manager remove-mapping --schema myschema
```

## Connection Options

Flags or environment variables:

- `--host` / `PGHOST` (default: localhost)
- `--port` / `PGPORT` (default: 5432)
- `--user` / `PGUSER` (required)
- `--password` / `PGPASSWORD` (required)
- `--dbname` / `PGDATABASE` (required for add/list/remove)

## Verbosity

- `-v`: Show SQL statements (excludes trigger function)
- `-vv`: Show all SQL including trigger function

## Notes

- All operations are idempotent
- Config table stored in `public` schema regardless of search_path
- User grants (`GRANT role TO user`) must be handled manually
