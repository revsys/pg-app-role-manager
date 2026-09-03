# Completed Tasks Archive

This document contains all completed tasks from the pg-app-role-manager project. See TODO.md for pending work.

## Project Decisions
- **Binary name**: `pg-app-role-manager`
- **Location**: Current directory (user-config/)
- **Dependencies**: All pure Rust (no external C libraries required)
- **Scope**: Per-database (config table and triggers in each database, not global)
- **Idempotency**: Skip and continue if objects exist
- **User grants**: NOT implemented (admins handle `GRANT role TO user` manually)

## Completed Work

### 1. Project Initialization
- [x] Initialize Cargo project with `cargo init --name pg-app-role-manager`
  - **Thinking Mode**: ❌ Not needed - straightforward command execution

- [x] Add dependencies to Cargo.toml (all pure Rust, no libpq needed)
  - clap (v4.5 with derive, env features)
  - tokio (v1 with full features)
  - tokio-postgres (v0.7 - native PostgreSQL protocol)
  - anyhow (v1.0)
  - chrono (v0.4 with clock)
  - postgres-types (v0.2 with with-chrono-0_4)
  - **Thinking Mode**: ❌ Not needed - standard dependencies

### 2. CLI Framework Setup (src/cli.rs)
- [x] Define CLI structure with clap
  - Main commands: `init`, `add-mapping`, `list-mappings`, `remove-mapping`
  - Global connection flags: `--host`, `--port`, `--user`, `--password`, `--dbname`
  - Init-specific flags: `--database`, `--schema`, `--role`
  - Command-specific flags: `--schema`, `--role` for add-mapping/remove-mapping
  - **Thinking Mode**: ⚠️ Minimal - deciding on exact flag names and structure

- [x] Implement environment variable fallback logic
  - Support PGHOST (default: localhost), PGPORT (default: 5432), PGUSER, PGPASSWORD, PGDATABASE
  - Implement precedence: CLI flags override env vars
  - **Thinking Mode**: ✅ Moderate - need to reason through precedence and validation logic

### 3. Database Connection Management (src/db.rs)
- [x] Create ConnectionConfig struct
  - Fields: host, port, user, password, dbname (optional for init)
  - **Thinking Mode**: ❌ Not needed - straightforward data structure

- [x] Implement build_connection_string() function
  - Construct postgres://user:password@host:port/dbname URI
  - Handle optional dbname (for init, connect to 'postgres' system db)
  - **Thinking Mode**: ⚠️ Minimal - straightforward but needs validation

- [x] Create connect() async function with tokio-postgres
  - Return tokio_postgres::Client
  - Handle connection errors with user-friendly messages
  - **Thinking Mode**: ⚠️ Minimal - mostly boilerplate error handling

### 4. SQL Templating (src/sql_templates.rs)
- [x] Create SQL template engine
  - Replace placeholders: {database}, {schema}, {role}
  - Use proper identifier quoting for PostgreSQL (format! with careful escaping)
  - Remove user grant logic (GRANT role TO user) from original SQL
  - Change config table location from public.schema_ownership_config to per-database
  - **Thinking Mode**: ✅ High - critical security consideration, need to reason through safe templating

- [x] Break SQL pattern into logical sections with existence checks
  - Section 1: Database creation (skip if exists)
  - Section 2: Schema creation (skip if exists)
  - Section 3: Role creation (skip if exists)
  - Section 4: Schema ownership and grants (idempotent)
  - Section 5: Config table creation (IF NOT EXISTS)
  - Section 6: Event trigger function (CREATE OR REPLACE)
  - Section 7: Event trigger creation (check existence first)
  - Section 8: Initial mapping (ON CONFLICT DO UPDATE)
  - **Thinking Mode**: ✅ Moderate - need to decide how to handle transaction boundaries and failures

### 5. Command: `init` (src/commands/init.rs)
- [x] Connect to 'postgres' system database
  - **Thinking Mode**: ❌ Not needed - straightforward connection

- [x] Implement database creation logic
  - Query pg_database to check existence
  - If exists: log "Database exists, continuing" and skip
  - If not: CREATE DATABASE {database}
  - **Thinking Mode**: ✅ Moderate - error handling and idempotency

- [x] Reconnect to target database
  - Disconnect from postgres, connect to newly created/existing database
  - **Thinking Mode**: ⚠️ Minimal - connection switching

- [x] Implement schema creation logic
  - Query pg_namespace to check existence
  - If exists: log and skip
  - If not: CREATE SCHEMA {schema}
  - **Thinking Mode**: ⚠️ Minimal - similar pattern to database

- [x] Implement role creation logic
  - Query pg_roles to check existence
  - If exists: log and skip
  - If not: CREATE ROLE {role} NOLOGIN
  - **Thinking Mode**: ✅ Moderate - role management

- [x] Implement schema ownership transfer and grants
  - ALTER SCHEMA {schema} OWNER TO {role}
  - GRANT USAGE, CREATE on schema
  - GRANT ALL on existing tables/sequences/functions
  - ALTER DEFAULT PRIVILEGES
  - **Thinking Mode**: ✅ Moderate - grant management complexity

- [x] Install schema_ownership_config table in current database
  - CREATE TABLE IF NOT EXISTS schema_ownership_config
  - Note: Per-database, not in public schema globally
  - **Thinking Mode**: ⚠️ Minimal - straightforward table creation

- [x] Install event trigger function
  - CREATE OR REPLACE FUNCTION auto_transfer_schema_ownership()
  - **Thinking Mode**: ⚠️ Minimal - function creation

- [x] Install event trigger
  - Check if trigger 'auto_transfer_schema_ownership_trigger' exists in pg_event_trigger
  - If not: CREATE EVENT TRIGGER
  - **Thinking Mode**: ✅ Moderate - PostgreSQL event trigger specifics

- [x] Insert initial mapping to schema_ownership_config
  - INSERT ... ON CONFLICT (schema_name) DO UPDATE
  - **Thinking Mode**: ❌ Not needed - simple INSERT with ON CONFLICT

### 6. Command: `add-mapping` (src/commands/add_mapping.rs)
- [x] Validate that schema exists
  - Query pg_namespace WHERE nspname = $1
  - Return error if not found
  - **Thinking Mode**: ⚠️ Minimal - basic validation queries

- [x] Validate that role exists
  - Query pg_roles WHERE rolname = $1
  - Return error if not found
  - **Thinking Mode**: ⚠️ Minimal - basic validation queries

- [x] Implement schema-to-role mapping insertion
  - INSERT INTO schema_ownership_config (schema_name, target_role)
  - Use ON CONFLICT (schema_name) DO UPDATE SET target_role = EXCLUDED.target_role
  - **Thinking Mode**: ❌ Not needed - straightforward SQL execution

### 7. Command: `list-mappings` (src/commands/list_mappings.rs)
- [x] Query schema_ownership_config table
  - SELECT * FROM schema_ownership_config ORDER BY schema_name
  - Display as formatted table (simple println! formatting, no extra dependencies)
  - **Thinking Mode**: ❌ Not needed - simple SELECT and formatting

### 8. Command: `remove-mapping` (src/commands/remove_mapping.rs)
- [x] Delete entry from schema_ownership_config
  - DELETE FROM schema_ownership_config WHERE schema_name = $1
  - Report number of rows affected
  - **Thinking Mode**: ❌ Not needed - simple DELETE

### 9. Error Handling
- [x] Use anyhow::Result throughout (opted for anyhow instead of custom error types)
  - Used .context() to add user-friendly messages to errors
  - Convert tokio-postgres errors to readable messages
  - **Thinking Mode**: ✅ High - designing good error hierarchy and messages

### 10. Main Entry Point (src/main.rs)
- [x] Create module structure
  - mod cli, db, sql_templates, commands
  - **Thinking Mode**: ❌ Not needed - straightforward structure

- [x] Implement async main with tokio
  - Parse CLI args
  - Match on command and dispatch to appropriate handler
  - **Thinking Mode**: ⚠️ Minimal - standard async main pattern

### 11. Manual Testing & Verification
- [x] Test init command on fresh database
  - Verify all objects created correctly
  - **Thinking Mode**: ❌ Not needed - manual testing

- [x] Test idempotency (run init twice)
  - Should skip existing objects without errors
  - **Thinking Mode**: ❌ Not needed - manual testing

- [x] Test event trigger functionality
  - Create table in managed schema, verify ownership transfers
  - **Thinking Mode**: ❌ Not needed - manual testing

- [x] Test add-mapping, list-mappings, remove-mapping commands
  - **Thinking Mode**: ❌ Not needed - manual testing

- [x] Test environment variable fallback
  - **Thinking Mode**: ❌ Not needed - manual testing

### 12. Documentation (Partial)
- [x] Write CHANGELOG.md
  - Version 0.1.0 initial release documentation
  - Bug fix for --database flag fallback to PGDATABASE
  - Comprehensive feature documentation
  - **Thinking Mode**: ❌ Not needed - documentation writing

### 13. Bug Fixes & Enhancements (Post-Initial Implementation)
- [x] Fix --database flag to properly fall back to PGDATABASE environment variable
  - Changed --database from required to optional in init command
  - Added database resolution logic in main.rs
  - Proper error message when neither flag nor env var is set
  - **Thinking Mode**: ⚠️ Minimal - straightforward flag handling

- [x] Improve list-mappings output formatting
  - Truncate target role at 30 characters with [...] indicator
  - Adjust column widths for better readability
  - Dynamic column formatting
  - **Thinking Mode**: ⚠️ Minimal - string formatting

### 14. Completion Report Feature
- [x] Add action tracking and summary report system
  - Created src/report.rs with ActionOutcome enum and ActionReport struct
  - ActionOutcome variants: Created, Skipped, Updated, Removed, NotFound
  - ActionReport.record() prints immediate output and collects results
  - ActionReport.print_summary() displays aggregated counts at end
  - **Thinking Mode**: ⚠️ Minimal - straightforward data collection

- [x] Instrument all commands with action reporting
  - init: Tracks 17 operations (Created/Skipped for conditional, Updated for grants)
  - add-mapping: Reports Updated for upsert operation
  - remove-mapping: Reports Removed or NotFound based on rows affected
  - list-mappings: Adds simple "Total mappings: N" count line
  - **Thinking Mode**: ⚠️ Minimal - mechanical changes to existing commands

- [x] Add graceful error handling for uninitialized databases
  - list-mappings: Detects missing table (SQLSTATE 42P01), prints friendly message
  - add-mapping: Returns error with init instruction if table missing
  - remove-mapping: Returns error with init instruction if table missing
  - All commands check for undefined_table error and provide actionable guidance
  - **Thinking Mode**: ⚠️ Minimal - error code checking and user-friendly messaging

- [x] Schema-qualify config table as public.schema_ownership_config
  - Updated CREATE TABLE statement in sql_templates.rs
  - Updated trigger function to query public.schema_ownership_config
  - Updated INSERT in insert_initial_mapping()
  - Updated all command queries (add-mapping, list-mappings, remove-mapping)
  - Prevents ambiguity when databases have custom search_path settings
  - **Thinking Mode**: ⚠️ Minimal - systematic find-and-replace with schema qualification

- [x] Add -v/-vv verbosity levels for SQL statement visibility
  - Changed from --verbose bool to -v count-based flag (u8) in cli.rs
  - Level 1 (-v): Shows all SQL statements except trigger function
  - Level 2 (-vv): Shows all SQL including trigger function
  - Updated all commands to use `verbose >= 1` or `verbose >= 2` checks
  - Output format: `[SQL] <statement> -- params: [<values>]` for parameterized queries
  - Trigger function only logged at level 2+ to reduce noise
  - **Thinking Mode**: ⚠️ Minimal - systematic addition of conditional logging with levels

### 15. Documentation and Static Build Support
- [x] Write README.md
  - Brief documentation covering build, usage, connection options, and verbosity
  - Build instructions for both standard and musl static binary
  - Command examples for all operations (init, add-mapping, list-mappings, remove-mapping)
  - Environment variable reference
  - **Thinking Mode**: ❌ Not needed - straightforward documentation

- [x] Add musl static build support
  - Added x86_64-unknown-linux-musl target
  - Enables fully static binary without libc dependencies
  - Build command: `cargo build --release --target x86_64-unknown-linux-musl`
  - **Thinking Mode**: ❌ Not needed - standard Rust cross-compilation

---

## Thinking Mode Summary (Reference)

**High Thinking** (complex reasoning required):
- SQL templating and injection safety ✓
- Error type design ✓
- Init command implementation (role/grant management) ✓
- SQL pattern breakdown and transaction handling ✓

**Moderate Thinking** (some reasoning needed):
- Connection precedence logic ✓
- Event trigger installation ✓
- Integration testing strategy ✓
- Error message mapping ✓

**Low/None Thinking** (straightforward execution):
- Project initialization ✓
- Dependency management ✓
- Simple CRUD operations (list, remove) ✓
- Documentation ✓
- Basic commands and formatting ✓

### 16. TLS/SSL Connection Implementation
**Completed:** January 2026
**Total Tasks:** 31 across 7 phases

- [x] **Phase 1: Dependencies and Type Definitions** ✓ COMPLETE
  - Added postgres_rustls, rustls, tokio-rustls, webpki-roots dependencies
  - Created SslMode enum (Disable, Prefer, Require) with from_str() validation
  - Implemented Default trait returning Prefer
  - **Thinking Mode**: ❌ Not needed - straightforward dependency and enum additions

- [x] **Phase 2: Configuration Updates** ✓ COMPLETE
  - Added sslmode field to ConnectionConfig struct
  - Added sslmode CLI flag with PGSSLMODE environment variable support
  - Updated main.rs to parse and pass sslmode to connection config
  - **Thinking Mode**: ❌ Not needed - mechanical field additions

- [x] **Phase 3: TLS Connector Implementation** ✓ COMPLETE
  - Created custom NoVerifier implementing ServerCertVerifier trait
  - Implements PostgreSQL "require" semantics: encryption without certificate verification
  - Built rustls ClientConfig with .dangerous().with_custom_certificate_verifier()
  - Set PostgreSQL ALPN protocol (critical for handshake)
  - Created MakeTlsConnector wrapping tokio-rustls TlsConnector
  - **Thinking Mode**: ✅ High - security-critical TLS configuration
  - **Key Decision**: No certificate verification matches PostgreSQL's "require" mode
  - **Security Note**: Provides encryption but not server identity verification

- [x] **Phase 4: Connection Logic Rewrite** ✓ COMPLETE
  - Implemented SslMode::Disable branch (NoTls, existing behavior)
  - Implemented SslMode::Require branch (TLS connector, no fallback)
  - Implemented SslMode::Prefer branch (try TLS first, fallback to NoTls on any error)
  - Added all necessary rustls imports for custom certificate verifier
  - **Thinking Mode**: ✅ High - complex error handling with fallback logic
  - **Key Decision**: All TLS errors trigger fallback in Prefer mode (matches PostgreSQL)

- [x] **Phase 5: Build and Basic Validation** ✓ COMPLETE
  - cargo check passed without errors
  - cargo build --release succeeded
  - Binary size: 7.2M (includes TLS stack)
  - **Thinking Mode**: ❌ Not needed - verification step

- [x] **Phase 6: Testing** ✓ COMPLETE
  - Tested require mode with SSL-enabled server (self-signed certificate)
  - Connection successful with TLS encryption
  - Tested prefer mode fallback logic
  - Warning message confirmed: "TLS connection failed (...), falling back to unencrypted connection"
  - Verified behavior with server requiring encryption (pg_hba.conf rejects unencrypted)
  - **Thinking Mode**: ❌ Not needed - manual testing
  - **Test Environment**: PostgreSQL server with self-signed certificate, encryption required

- [x] **Phase 7: Documentation and Cleanup** ✓ COMPLETE
  - Updated TODO.md to mark TLS implementation complete
  - Documented PostgreSQL semantics match
  - Added build target requirement (x86_64-unknown-linux-musl)
  - Moved completed work to ARCHIVE.md
  - **Thinking Mode**: ❌ Not needed - documentation

**Implementation Notes:**
- **PostgreSQL Semantics**: Matches PostgreSQL's sslmode behavior:
  - `disable`: No TLS encryption
  - `prefer`: Try TLS first, fallback to unencrypted if TLS fails (default)
  - `require`: Require TLS encryption, no certificate verification
- **Not Implemented**: verify-ca and verify-full modes (certificate validation)
- **Custom Verifier**: NoVerifier accepts all certificates without validation
- **Security Trade-off**: Prevents passive eavesdropping but not active MITM attacks
- **ALPN Protocol**: Correctly sets "postgresql" ALPN identifier (required for handshake)

**Technical Decisions:**
1. Used rustls instead of native-tls for pure Rust implementation
2. Implemented custom ServerCertVerifier to bypass certificate checks
3. All TLS errors in Prefer mode trigger fallback (simple, matches PostgreSQL)
4. No certificate validation in any mode (matches PostgreSQL "require" semantics)

### 17. Command Simplification - Removal of add-mapping and remove-mapping
**Completed:** January 2026

**Decision:** Removed add-mapping and remove-mapping commands to avoid excessive complexity

**Rationale:**
- Managing mutable schema-to-role mappings introduced corner cases:
  * Ownership transfers when changing roles
  * Cleanup of privileges, default privileges, triggers, and functions on removal
  * Potential for inconsistent state
  * Risk of accidentally breaking production schemas
- Original design was optimistic about handling all edge cases
- Simpler is better: establish mappings once during init, leave them immutable

**Changes Made:**
- [x] Removed AddMapping and RemoveMapping variants from CLI enum (src/cli.rs)
- [x] Deleted src/commands/add_mapping.rs
- [x] Deleted src/commands/remove_mapping.rs
- [x] Updated main.rs to remove command dispatch logic
- [x] Updated src/commands/mod.rs to remove module declarations
- [x] Cleaned up src/report.rs - removed unused ActionOutcome::Removed and ActionOutcome::NotFound
- [x] Updated TODO.md with new project decisions
- [x] Documented change in ARCHIVE.md

**New Design:**
- Schema-to-role mappings established only via `init` command
- Mappings are **immutable** after initialization
- `list-mappings` retained for visibility into current configuration
- Event trigger and config table remain database-wide
- If mapping changes needed: manual SQL or drop/recreate database

**Thinking Mode:** ⚠️ Minimal - mechanical removal of code

**Testing:**
- cargo check: passed without warnings
- cargo build --release: successful
- --help output: only shows init and list-mappings commands
- Binary size: 7.2M (unchanged)

### Rehome: Configurable Source Schema (2026-09-03)

`rehome` previously assumed the objects being moved always lived in `public`
— every catalog query and every `ALTER ... SET SCHEMA` statement in
`src/commands/rehome.rs` hardcoded `'public'` as the source, and the CLI had
no source-schema flag at all (`--schema` actually meant the *target*).

**Changes (intentional breaking change to `rehome`'s CLI surface):**
- [x] `src/cli.rs`: `Command::Rehome` now takes `--source-schema` (required)
      and `--target-schema` (required, renamed from `--schema`) instead of a
      single `--schema` meaning target
- [x] `src/cli.rs` tests: renamed/updated `rehome_parses_schemas_without_database`,
      `rehome_with_database`; split `rehome_requires_schema` into
      `rehome_requires_source_schema`, `rehome_requires_target_schema`,
      `rehome_requires_schemas_when_both_missing`
- [x] `src/main.rs`: `Command::Rehome` match arm destructures and forwards
      both `source_schema` and `target_schema` to `commands::rehome::execute`
- [x] `src/commands/rehome.rs`: `execute()` gains `source_schema: String`;
      bails early if `source_schema == target_schema`; validates
      `source_schema` exists via the existing `schema_exists()` helper
      (same pattern as the target-schema check)
- [x] All six `move_*` functions (`move_types`, `move_tables`,
      `move_sequences`, `move_views`, `move_matviews`, `move_functions`)
      gain a `source_schema: &str` parameter; catalog queries now use a
      parameterized `WHERE ... = $1` bound to `source_schema` instead of a
      hardcoded `'public'` literal; `quote_identifier("public")` calls
      building the source side of each `ALTER ... SET SCHEMA` become
      `quote_identifier(source_schema)`; all error/report strings now
      interpolate the actual source schema instead of the literal "public"
- [x] Kept `tablename != 'schema_ownership_config'` (`move_tables`) and
      `proname != 'auto_transfer_schema_ownership'` (`move_functions`)
      unconditional and unparameterized — these protect the `init`-created
      ownership-transfer machinery, which always lives in `public`
      regardless of target schema

**Thinking Mode:** ✅ Moderate — required deciding CLI flag naming/breaking
change scope with the user, and choosing parameterized queries over string
interpolation for the now user-controlled source schema value.

**Testing:**
- `cargo build` and `cargo build --target x86_64-unknown-linux-musl`: clean
- `cargo test`: 83 passed, 0 failed

### Read-Only Role Creation in `init`

- [x] Add private `readonly_role()` helper to `SqlTemplates` (`src/sql_templates.rs`) — returns quoted `"{schema}_ro"`
- [x] Add `create_readonly_role()` — `CREATE ROLE "{schema}_ro" NOLOGIN`
- [x] Add `grant_connect_readonly()` — `GRANT CONNECT ON DATABASE "db" TO "{schema}_ro"`
- [x] Add `grant_schema_usage_readonly()` — `GRANT USAGE ON SCHEMA "schema" TO "{schema}_ro"`
- [x] Add `grant_select_tables()` — `GRANT SELECT ON ALL TABLES IN SCHEMA "schema" TO "{schema}_ro"`
- [x] Add `grant_select_sequences()` — `GRANT SELECT ON ALL SEQUENCES IN SCHEMA "schema" TO "{schema}_ro"`
- [x] Add `alter_default_privileges_select_tables()` — `ALTER DEFAULT PRIVILEGES IN SCHEMA "schema" GRANT SELECT ON TABLES TO "{schema}_ro"`
- [x] Add `alter_default_privileges_select_sequences()` — `ALTER DEFAULT PRIVILEGES IN SCHEMA "schema" GRANT SELECT ON SEQUENCES TO "{schema}_ro"`
- [x] Add read-only role existence check + creation block to `src/commands/init.rs`, after main role creation, using the existing `role_exists()` helper
- [x] Execute the six read-only grant statements in `init` and record to `ActionReport`
- [x] Unit tests for new `SqlTemplates` methods — verify `_ro` suffix appears, no INSERT/UPDATE/DELETE/CREATE/DROP SQL present

### Implement `ALTER DATABASE ... SET search_path`

- [x] Add `alter_database_search_path(db: &str, schema: &str) -> String` to `SqlTemplates`
      in `src/sql_templates.rs` — returns `ALTER DATABASE "db" SET search_path TO "schema"`
- [x] Call it during `init` after the schema is created, so the database-level default
      search path is set to the new app schema
- [x] Add unit test: verify the generated SQL contains both quoted identifiers

### Test Coverage

Tests live in `#[cfg(test)]` modules inside their respective source files.

#### Prep: extract shared `quote_identifier`

- [x] Move the `quote_identifier` logic out of both `SqlTemplates` (method) and
  `rehome.rs` (free function) into a single `pub(crate) fn quote_identifier(name: &str) -> String`
  in a new `src/utils.rs` module. Update callers.

#### `src/utils.rs` — `quote_identifier`

- [x] **Normal identifier** — `quote_identifier("foo")` → `"\"foo\""`
- [x] **Empty string** — `quote_identifier("")` → `"\"\""`
- [x] **Name containing a double quote** — `quote_identifier("fo\"o")` → `"\"fo\"\"o\""` (quote is doubled)
- [x] **Name containing two consecutive double quotes** — verify both are escaped
- [x] **Name that is already a reserved word** — treated as an opaque string; verify output is just wrapped

#### `src/sql_templates.rs` — `SqlTemplates`

- [x] **`create_database`** — output is `CREATE DATABASE "mydb"`
- [x] **`create_schema`** — output is `CREATE SCHEMA "myschema"`
- [x] **`create_role`** — output is `CREATE ROLE "myrole" NOLOGIN`
- [x] **`grant_connect`** — output contains both quoted database and role names
- [x] **`alter_schema_owner`** — output contains `ALTER SCHEMA "s" OWNER TO "r"`
- [x] **`grant_schema_usage`** — output contains `GRANT USAGE ON SCHEMA`
- [x] **`grant_schema_create`** — output contains `GRANT CREATE ON SCHEMA`
- [x] **`grant_all_tables`** — output contains `GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA`
- [x] **`grant_all_sequences`** — output contains `GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA`
- [x] **`grant_all_functions`** — output contains `GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA`
- [x] **`alter_default_privileges_tables`** — output contains `ALTER DEFAULT PRIVILEGES IN SCHEMA`
- [x] **`alter_default_privileges_sequences`** — output contains `SEQUENCES`
- [x] **`alter_default_privileges_functions`** — output contains `FUNCTIONS`
- [x] **`create_config_table`** — static str contains `schema_ownership_config`, `schema_name`, `target_role`, `created_at`, `updated_at`
- [x] **`create_trigger_function`** — static str contains `auto_transfer_schema_ownership`, `SECURITY DEFINER`, `pg_event_trigger_ddl_commands`, all handled object types
- [x] **`create_event_trigger`** — static str contains `auto_transfer_schema_ownership_trigger`, `ddl_command_end`
- [x] **`insert_initial_mapping`** — output contains both schema and role values, `ON CONFLICT`
- [x] **Injection via schema name with embedded quote** — quote is doubled (not raw)
- [x] **Injection via role name with embedded quote** — same exercise for `create_role()`

#### `src/report.rs` — `ActionOutcome` and `ActionReport`

- [x] **`ActionOutcome::Created` display** — `format!("{}", ActionOutcome::Created)` == `"Created"`
- [x] **`ActionOutcome::Skipped` display** — `"Skipped"`
- [x] **`ActionOutcome::Updated` display** — `"Updated"`
- [x] **`ActionOutcome::Moved` display** — `"Moved"`
- [x] **`ActionReport` empty summary** — zero actions → `Total actions: 0`, no per-type lines
- [x] **`ActionReport` counts one of each** — summary shows `Total actions: 4`, `Created: 1`, `Skipped: 1`, `Updated: 1`, `Moved: 1`
- [x] **`ActionReport` omits zero-count types** — record only Moved; summary must NOT contain `"Created:"`, `"Skipped:"`, or `"Updated:"` lines

#### `src/db.rs` — `SslMode` and `ConnectionConfig`

- [x] **`SslMode::from_str("disable")`** — returns `Ok(SslMode::Disable)`
- [x] **`SslMode::from_str("prefer")`** — returns `Ok(SslMode::Prefer)`
- [x] **`SslMode::from_str("require")`** — returns `Ok(SslMode::Require)`
- [x] **`SslMode::from_str` is case-insensitive** — `"DISABLE"`, `"Prefer"`, `"REQUIRE"` all succeed
- [x] **`SslMode::from_str` rejects unknown values** — `"verify-full"`, `""`, `"tls"` return `Err`
- [x] **`SslMode::default()`** — returns `SslMode::Prefer` (matches the clap default)
- [x] **`ConnectionConfig::build_connection_string` with dbname** — output contains `dbname=mydb`
- [x] **`ConnectionConfig::build_connection_string` without dbname** — output contains `dbname=postgres` (fallback)

#### `src/commands/list_mappings.rs` — `truncate_with_ellipsis`

- [x] **String shorter than max** — returned unchanged
- [x] **String exactly equal to max** — returned unchanged
- [x] **String one byte over max** — truncated; result ends with `"[...]"`
- [x] **Long string** — truncated version is no longer than `max_len`
- [x] **max_len ≤ 5** — `saturating_sub(5)` produces 0; does not panic

#### `src/cli.rs` — argument parsing

- [x] **Minimum valid invocation: `init`** — parses successfully; user/password/schema/role fields correct
- [x] **Default host and port** — not passing `--host`/`--port`; `connection.host == "localhost"`, `connection.port == 5432`
- [x] **Default sslmode** — not passing `--sslmode`; `connection.sslmode == "prefer"`
- [x] **Verbosity counting** — `-v` → `verbose == 1`; `-vv` → `verbose == 2`; not passed → `verbose == 0`
- [x] **`list-mappings` subcommand** — parses without `--schema`/`--role`
- [x] **`rehome` subcommand** — requires `--schema`; passes without `--database` (predates the source/target schema split — see "Rehome: Configurable Source Schema" above)
- [x] **`version` subcommand** — parses successfully
- [x] **Missing `--user` is rejected** — `try_parse_from` returns `Err`
- [x] **Missing `--password` is rejected** — `try_parse_from` returns `Err`
- [x] **`init` missing `--schema` is rejected** — `Err`
- [x] **`init` missing `--role` is rejected** — `Err`
- [x] **`rehome` missing `--schema` is rejected** — `Err` (predates the source/target schema split)

### Command Simplification (January 2026)
**Removed commands:** add-mapping, remove-mapping

**Rationale:** Managing multiple roles per schema introduced excessive complexity:
- Corner cases with ownership transfers
- Cleanup logic for privileges, triggers, and functions
- Potential for inconsistent state

**New design:**
- Schema-to-role mappings are established only via `init` command
- Mappings are **immutable** after initialization
- Simpler mental model: one schema → one role, set once
- `list-mappings` remains for visibility into current state

**Files removed:**
- `src/commands/add_mapping.rs`
- `src/commands/remove_mapping.rs`

**Files updated:**
- `src/cli.rs` - Removed AddMapping and RemoveMapping variants
- `src/main.rs` - Removed command dispatch logic
- `src/commands/mod.rs` - Removed module declarations
- `src/report.rs` - Removed unused ActionOutcome variants (Removed, NotFound)

### TLS/SSL Connection Implementation

**Status: All phases complete and tested successfully**

Implementation matches PostgreSQL semantics:
- **disable**: No TLS encryption
- **prefer** (default): Try TLS first, fallback to unencrypted if TLS fails
- **require**: Require TLS encryption (no certificate verification)

Note: Unlike standard PostgreSQL, verify-ca and verify-full modes are not implemented.

#### Phase 1: Dependencies and Type Definitions
- [x] **Update Cargo.toml dependencies** [STRAIGHTFORWARD] ✓ COMPLETED
  - ✓ tokio-postgres features: `{ version = "0.7", features = ["runtime"] }`
  - ✓ postgres_rustls = "0.1" (provides TLS connector for tokio-postgres)
  - ✓ rustls-webpki = "0.102" (certificate validation)
  - Complexity: LOW - Simple dependency additions
  - Deep thinking: NOT REQUIRED - Follow established pattern
  - Note: Used postgres_rustls instead of tokio-postgres-rustls per actual crate availability

- [x] **Create SslMode enum in src/db.rs** [STRAIGHTFORWARD] ✓ COMPLETED
  - ✓ Add enum with variants: Disable, Prefer, Require
  - ✓ Implement from_str() with validation (case-insensitive, helpful error messages)
  - ✓ Implement Default trait (returns Prefer)
  - ✓ Add Clone and Debug derives
  - Complexity: LOW - Standard enum pattern
  - Deep thinking: NOT REQUIRED - Well-defined specification

#### Phase 2: Configuration Updates
- [x] **Add sslmode field to ConnectionConfig in src/db.rs** [TRIVIAL] ✓ COMPLETED
  - ✓ Add `pub sslmode: SslMode` field (line 38)
  - Complexity: TRIVIAL - Single field addition
  - Deep thinking: NOT REQUIRED
  - Verified: Compilation error in main.rs:16 as expected

- [x] **Add sslmode field to ConnectionOpts in src/cli.rs** [STRAIGHTFORWARD] ✓ COMPLETED
  - ✓ Add field with clap attributes: `#[arg(long, env = "PGSSLMODE", default_value = "prefer")]`
  - ✓ Add help text: "SSL mode: disable, prefer, or require"
  - ✓ Field type: String (parsed to SslMode in main.rs)
  - Complexity: LOW - Standard clap pattern
  - Deep thinking: NOT REQUIRED - Clear specification

- [x] **Update main.rs to parse and pass sslmode** [STRAIGHTFORWARD] ✓ COMPLETED
  - ✓ Import SslMode from db module (line 10)
  - ✓ Call SslMode::from_str() with error handling (line 17)
  - ✓ Pass sslmode to ConnectionConfig construction (line 25)
  - Complexity: LOW - Straightforward integration
  - Deep thinking: NOT REQUIRED - Clear integration point
  - Verified: Invalid values rejected with helpful error, valid values accepted, env var honored

#### Phase 3: TLS Connector Implementation (Detailed Breakdown)

**Overview:** Create TLS connector helper function with proper certificate validation
**Total Steps:** 6 (2 trivial, 2 straightforward, 2 moderate)
**Security-Critical:** YES - Certificate validation affects connection security

##### Step 3.1: Fix Cargo.toml Dependencies [TRIVIAL]
- [x] **Add webpki-roots dependency**
  - Add `webpki-roots = "0.26"` to [dependencies]
  - Purpose: Provides Mozilla's root CA certificates for server validation
  - Complexity: TRIVIAL - Single line addition
  - Deep thinking: NOT REQUIRED - Dependency add is mechanical
  - **Why this matters:** Without root CAs, all server certificates will be rejected
  - ✓ Also added rustls = "0.23" and tokio-rustls = "0.26" (required for implementation)

- [x] **Remove rustls-webpki dependency**
  - Remove `rustls-webpki = "0.102"` line
  - Reason: Pulled in transitively by rustls v0.23 (correct version 0.103)
  - Complexity: TRIVIAL - Single line removal
  - Deep thinking: NOT REQUIRED - Cleanup task
  - ✓ Removed successfully

##### Step 3.2: Add Required Imports to src/db.rs [STRAIGHTFORWARD]
- [x] **Add postgres_rustls imports**
  - Add: `use postgres_rustls::MakeTlsConnector;`
  - Purpose: Main TLS connector type for PostgreSQL
  - Complexity: LOW - Standard import
  - Deep thinking: NOT REQUIRED
  - ✓ Added at line 2

- [x] **Add rustls imports**
  - Add: `use rustls::RootCertStore;`
  - Purpose: Certificate store for validation
  - Complexity: LOW - Standard import
  - Deep thinking: NOT REQUIRED
  - ✓ Added at line 3

- [x] **Add std imports**
  - Add: `use std::sync::Arc;`
  - Purpose: Share ClientConfig across connections
  - Complexity: LOW - Standard import
  - Deep thinking: NOT REQUIRED
  - **Why Arc?** ClientConfig is expensive to clone; Arc provides cheap reference counting
  - ✓ Added at line 4

##### Step 3.3: Create Function Skeleton [STRAIGHTFORWARD]
- [x] **Define create_tls_connector() function signature**
  - Signature: `fn create_tls_connector() -> Result<MakeTlsConnector>`
  - Placement: After ConnectionConfig impl, before connect() function
  - Visibility: Private (not pub) - internal helper only
  - Complexity: LOW - Function declaration
  - Deep thinking: NOT REQUIRED
  - Note: Returns Result for consistency, though current impl won't error
  - ✓ Function created at line 54

##### Step 3.4: Initialize Root Certificate Store [MODERATE - REQUIRES THOUGHT]
- [x] **Create empty RootCertStore**
  - Code: `let mut root_store = RootCertStore::empty();`
  - Complexity: LOW - API call
  - Deep thinking: NOT REQUIRED
  - ✓ Implemented at line 56

- [x] **Load webpki-roots certificates**
  - Code: `root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());`
  - Purpose: Load Mozilla's curated root CA certificates
  - Complexity: MODERATE - Security-critical
  - Deep thinking: RECOMMENDED
  - **Security considerations:**
    * ✓ webpki-roots = Industry-standard, well-maintained CA bundle
    * ✗ Empty store = Would reject ALL certificates (security misconfiguration)
    * ✗ Accept all = Security vulnerability
    * ✓ Decision: Use webpki-roots (compile-time bundled, no runtime IO)
  - **Why .iter().cloned()?** webpki_roots provides static data; need owned copies for RootCertStore
  - ✓ Implemented at lines 57-61 with proper certificate loading

##### Step 3.5: Build rustls ClientConfig [MODERATE - REQUIRES THOUGHT]
- [x] **Create ClientConfig with certificate validation**
  - Code: `let mut config = rustls::ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth();`
  - Complexity: MODERATE - Security-critical API
  - Deep thinking: REQUIRED
  - **Critical decisions:**
    1. **Root certificates:** Using root_store from previous step ✓
    2. **Client auth:** `.with_no_client_auth()` - we don't use client certificates
       - Future: Could add `.with_client_cert_resolver()` for mutual TLS
       - Current scope: Server-only validation
    3. **Cipher suites:** Using rustls defaults (secure, modern TLS 1.2+)
    4. **Protocol versions:** Using rustls defaults (TLS 1.2, 1.3)
  - **Why mutable?** Need to modify config in next step (ALPN)
  - ✓ Implemented at lines 64-66 with proper security configuration

- [x] **Set PostgreSQL ALPN protocol [CRITICAL]**
  - Code: `postgres_rustls::set_postgresql_alpn(&mut config);`
  - Purpose: Set Application-Layer Protocol Negotiation to "postgresql"
  - Complexity: LOW - API call, but CRITICAL to remember
  - Deep thinking: MINIMAL - but MUST NOT FORGET
  - **Critical importance:**
    * ✓ PostgreSQL servers require ALPN = "postgresql"
    * ✗ If omitted: TLS handshake will fail with cryptic errors
    * ✓ postgres_rustls provides helper function for this
    * This is NON-NEGOTIABLE - always required
  - **What it does:** Clears any existing ALPN values, sets to b"postgresql"
  - ✓ Implemented at line 69 - ALPN correctly set

##### Step 3.6: Create and Wrap TLS Connector [STRAIGHTFORWARD]
- [x] **Create tokio-rustls TlsConnector**
  - Code: `let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(config));`
  - Purpose: Bridge between rustls config and tokio async runtime
  - Complexity: LOW - Standard pattern
  - Deep thinking: NOT REQUIRED
  - **Why Arc::new()?** TlsConnector expects Arc<ClientConfig> for sharing
  - **Performance:** Arc allows cheap cloning across connections
  - ✓ Implemented at line 72

- [x] **Wrap in postgres_rustls MakeTlsConnector**
  - Code: `Ok(MakeTlsConnector::new(tls_connector))`
  - Purpose: Adapt tokio-rustls to tokio-postgres TLS interface
  - Complexity: LOW - Final wrapping step
  - Deep thinking: NOT REQUIRED
  - **Why MakeTlsConnector?** tokio-postgres expects this specific trait
  - ✓ Implemented at line 73

##### Step 3.7: Verification [VALIDATION]
- [x] **Run cargo check**
  - Verify: No compilation errors in create_tls_connector()
  - Verify: All imports resolve correctly
  - Expected warnings: "function is never used" (until Phase 4)
  - Complexity: N/A - Validation
  - Deep thinking: CONDITIONAL - Only if errors occur
  - ✓ Compilation successful with expected warnings (function unused until Phase 4)

- [x] **Review implementation against checklist**
  - [x] Root certificates loaded from webpki-roots ✓
  - [x] ALPN set to "postgresql" ✓
  - [x] ClientConfig wrapped in Arc ✓
  - [x] Function returns Result<MakeTlsConnector> ✓
  - [x] No client certificate authentication ✓
  - Complexity: LOW - Checklist review
  - Deep thinking: NOT REQUIRED
  - ✓ All checklist items verified successfully

**Phase 3 Summary:**
- **Total substeps:** 13 implementation tasks + 1 verification
- **Trivial:** 2 (Cargo.toml changes)
- **Straightforward:** 8 (imports, function skeleton, wrapping)
- **Moderate (requires thought):** 3 (certificate store, ClientConfig, ALPN)
- **Deep thinking required for:** Steps 3.4, 3.5 (security implications)
- **Critical gotcha:** Must call set_postgresql_alpn() - easy to forget, hard to debug

**Security Review Points:**
1. ✓ Use industry-standard root CAs (webpki-roots)
2. ✓ Enable proper certificate validation (with_root_certificates)
3. ✓ Set PostgreSQL ALPN (required for handshake)
4. ✓ Use secure defaults (rustls handles cipher suites, protocols)
5. ✓ No client auth in initial implementation (scope limitation)

**Common Pitfalls to Avoid:**
- ✗ Forgetting to load root certificates → all connections fail
- ✗ Forgetting ALPN → mysterious TLS handshake failures
- ✗ Not using Arc → type errors
- ✗ Wrong crate name (postgres-rustls vs postgres_rustls) → import errors

#### Phase 4: Connection Logic Rewrite
- [x] **Implement SslMode::Disable branch in connect()** [STRAIGHTFORWARD] ✓ COMPLETED
  - ✓ Keep existing NoTls logic (lines 80-91)
  - ✓ Added match statement on config.sslmode
  - ✓ Moved existing connection code into SslMode::Disable arm
  - ✓ Added todo!() placeholders for Require and Prefer modes
  - Complexity: LOW - Preserve existing code
  - Deep thinking: NOT REQUIRED - No changes to current behavior
  - Verified: Disable mode works, Require/Prefer panic with "not yet implemented"

- [x] **Implement SslMode::Require branch in connect()** [MODERATE] ✓ COMPLETED
  - ✓ Create TLS connector using create_tls_connector()? (line 94)
  - ✓ Attempt connection with TLS connector instead of NoTls (line 96)
  - ✓ Use context() for clear error messages: "with required TLS" (line 98)
  - ✓ Spawn connection task (lines 100-104)
  - ✓ Return Ok(client) (line 106)
  - Complexity: MODERATE - Similar to existing pattern but with TLS
  - Deep thinking: MINIMAL - Straightforward TLS-only path
  - Verified: TLS connection succeeds, handshake completes, reaches authentication phase

- [x] **Implement SslMode::Prefer branch in connect()** [COMPLEX] ✓ COMPLETED
  - ✓ Create TLS connector
  - ✓ Attempt TLS connection first
  - ✓ On TLS failure: capture error, log warning, attempt NoTls fallback
  - ✓ Ensure both paths spawn connection task properly
  - ✓ Handle nested Result/Error types correctly
  - Complexity: HIGH - Branching logic with fallback
  - Deep thinking: REQUIRED - Error handling complexity
  - Decision: All TLS errors trigger fallback (matches PostgreSQL prefer semantics)
  - Verified: Successfully tested with TLS-required server

- [x] **Update imports in src/db.rs** [TRIVIAL] ✓ COMPLETED
  - ✓ All necessary imports added for custom certificate verifier
  - ✓ Imports: ServerCertVerifier, HandshakeSignatureValid, ServerCertVerified
  - ✓ Imports: CertificateDer, ServerName, UnixTime, DigitallySignedStruct, SignatureScheme
  - Complexity: TRIVIAL - Standard imports
  - Deep thinking: NOT REQUIRED

#### Phase 5: Build and Basic Validation
- [x] **Run cargo check** [VERIFICATION] ✓ COMPLETED
  - ✓ No compilation errors
  - ✓ All type checking passed
  - Complexity: N/A - Validation step

- [x] **Run cargo build --release** [VERIFICATION] ✓ COMPLETED
  - ✓ Release build succeeded
  - ✓ Binary size: 7.2M
  - Complexity: N/A - Validation step

#### Phase 6: Testing
- [x] **Test require mode** [VERIFICATION] ✓ COMPLETED
  - ✓ Tested with SSL-enabled server (self-signed cert)
  - ✓ Connection successful with TLS encryption
  - ✓ No certificate verification (matches PostgreSQL require semantics)
  - Complexity: LOW - Simple verification
  - Note: Server required encryption; unencrypted connections rejected by pg_hba.conf

- [x] **Test prefer mode (default)** [VERIFICATION] ✓ COMPLETED
  - ✓ Tested with SSL-enabled server
  - ✓ TLS attempted first, fallback logic works correctly
  - ✓ Warning message displays on fallback: "TLS connection failed (...), falling back to unencrypted connection"
  - ✓ Successful connection with require mode
  - Complexity: MODERATE - Multiple scenarios tested

#### Phase 7: Documentation and Cleanup
- [x] **Update TODO.md** [TRIVIAL] ✓ COMPLETED
  - ✓ Marked TLS implementation as complete
  - ✓ Documented PostgreSQL semantics match
  - ✓ Added build target requirement (x86_64-unknown-linux-musl)
  - Complexity: TRIVIAL - Documentation

#### Summary of Complexity Analysis

**Trivial tasks (10)**: Mechanical changes, no decisions
  - Phase 1: 2 (Cargo.toml, SslMode enum)
  - Phase 2: 3 (struct field, CLI field, main.rs update)
  - Phase 3: 2 (dependency changes)
  - Phase 4: 1 (imports)
  - Phase 7: 1 (documentation)
  - Verification: 1 (cargo check)

**Straightforward tasks (14)**: Standard patterns, minimal thought
  - Phase 3: 8 (imports, function skeleton, wrapping)
  - Phase 4: 2 (Disable branch, Require branch)
  - Verification: 4 (build, tests)

**Moderate tasks (5)**: Careful attention to API usage, security considerations
  - Phase 3: 3 (certificate store, ClientConfig, ALPN)
  - Phase 4: 1 (imports - actually moved to straightforward)
  - Verification: 1 (prefer mode testing)

**Complex tasks (1)**: Deep thinking required for error handling
  - Phase 4: 1 (Prefer mode fallback logic)

**Critical thinking required for:**
1. **Phase 3, Steps 3.4-3.5:** TLS connector creation (security-critical certificate validation)
2. **Phase 4:** Prefer mode fallback logic (complex error handling with nested Results)

**Total tasks:** 31 implementation + verification tasks
- **Completed:** 31/31 (100%) ✓
- **Status:** TLS/SSL implementation complete and production-ready
