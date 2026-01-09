# Changelog

All notable changes to pg-app-role-manager will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-01-09

### Breaking Changes
- **Removed Commands**: `add-mapping` and `remove-mapping` commands removed
  - Schema-to-role mappings are now **immutable** after initialization
  - Rationale: Avoiding complex ownership transfer logic and potential for inconsistent state
  - Migration: Use `init` to establish mappings; manual SQL required to change existing mappings
- **API Change**: `list-mappings` no longer accepts `--dbname` flag
  - Now scans all non-system databases automatically
  - Displays database name in output table

### Added
- **TLS/SSL Support**: Full encryption support matching PostgreSQL semantics
  - `--sslmode disable`: No encryption
  - `--sslmode prefer` (default): Try TLS, fallback to unencrypted if TLS fails
  - `--sslmode require`: Require TLS encryption (no certificate verification)
  - Environment variable: `PGSSLMODE`
  - Implementation uses rustls with custom certificate verifier
  - No verify-ca or verify-full modes (certificate validation not implemented)
- **Multi-Database Scanning**: `list-mappings` now queries all user databases
  - Automatically discovers non-system databases via `pg_database`
  - Output includes database name column
  - Summary shows total mappings across all databases
  - Gracefully handles connection failures and missing config tables
- **System Database Protection**: Blocks operations on system databases
  - PostgreSQL core: postgres, template0, template1
  - AWS RDS: rdsadmin
  - Azure: azure_maintenance
  - GCP Cloud SQL: cloudsqladmin
  - Applies to both `init` and `list-mappings` commands
- **Password Security**: Password values hidden in help output
  - `--password` flag uses `hide_env_values = true`
  - Environment variable name still shown, but value obscured
  - Prevents credential leakage in screenshots and documentation
- **Completion Reports**: All commands display summary report upon completion
  - Shows aggregated counts of actions (Created, Skipped, Updated)
  - Provides clear feedback about operations performed
- **Verbosity Levels**: Added `-v` and `-vv` flags for SQL visibility
  - `-v`: Shows SQL statements (excludes trigger function)
  - `-vv`: Shows all SQL including trigger function definition
- **Static Binary Support**: x86_64-unknown-linux-musl target
  - Fully static binary, no libc dependency
  - Portable across Linux distributions
  - Required build target per project standards
- **README Documentation**: Comprehensive usage documentation

### Changed
- **Config Table Location**: Schema-qualified as `public.schema_ownership_config`
  - Prevents ambiguity with custom `search_path` settings
  - Ensures consistent behavior regardless of session configuration
- **Report Output**: Removed "Removed" and "NotFound" action types
  - Only relevant action types retained: Created, Skipped, Updated

### Fixed
- `init` command properly falls back to `PGDATABASE` environment variable
- Clear error messages when required parameters missing
- **Graceful Handling of Uninitialized Databases**:
  - `list-mappings`: Shows friendly message when config table doesn't exist
  - Skips databases without schema_ownership_config during multi-database scan
  - All commands detect missing table (SQLSTATE 42P01) and provide guidance

## [0.1.0] - 2026-01-08

### Added

#### Core Functionality
- **CLI Framework**: Full-featured command-line interface with four main commands
  - `init`: Initialize database, schema, role, and automatic ownership transfer system
  - `add-mapping`: Add schema-to-role ownership mappings
  - `list-mappings`: Display all configured schema ownership mappings
  - `remove-mapping`: Remove schema-to-role ownership mappings

#### Database Management
- **Automatic Schema Ownership Transfer**: PostgreSQL event trigger system that automatically transfers ownership of newly created objects to configured roles
- **Idempotent Operations**: All database operations check for existing objects and skip gracefully
- **Per-Database Configuration**: Schema ownership configuration stored in each database separately (not globally)

#### Connection Management
- **PostgreSQL Standard Environment Variables Support**:
  - `PGHOST` (default: localhost)
  - `PGPORT` (default: 5432)
  - `PGUSER` (required)
  - `PGPASSWORD` (required)
  - `PGDATABASE` (optional, context-dependent)
- **CLI Flag Overrides**: Command-line flags take precedence over environment variables
- **Pure Rust Implementation**: No external C library dependencies (uses tokio-postgres with native protocol)

#### Schema Ownership Pattern Implementation
- **Database Creation**: Creates target database if it doesn't exist
- **Schema Creation**: Creates specified schema with proper ownership
- **Role Management**: Creates NOLOGIN roles for schema management
- **Comprehensive Permission Grants**:
  - `CONNECT` on database
  - `USAGE` and `CREATE` on schema
  - `ALL PRIVILEGES` on tables, sequences, and functions
  - Default privilege alterations for future objects
- **Event Trigger Installation**: Installs trigger function and event trigger for automatic ownership transfer
- **Configuration Table**: Creates `schema_ownership_config` table to store schema-to-role mappings

#### Event Trigger Features
- **Object Type Support**: Handles tables, sequences, views, materialized views, functions, and types
- **Smart Ownership Transfer**: Only transfers ownership if current owner differs from target role
- **Short-Circuit Logic**: Skips transfer if object already owned by target role
- **Security**: Runs with `SECURITY DEFINER` for proper privilege elevation

#### SQL Security
- **Identifier Quoting**: Proper PostgreSQL identifier quoting to prevent SQL injection
- **Parameterized Queries**: Uses prepared statements where applicable
- **Safe Template Rendering**: Careful escaping in SQL template generation

#### Error Handling
- **User-Friendly Error Messages**: Context-rich error reporting using anyhow
- **Connection Error Handling**: Clear messages for connection failures
- **Operation Failure Context**: Each database operation includes descriptive error context

### Technical Details

#### Dependencies
- `clap` v4.5 - Command-line argument parsing with derive and environment variable support
- `tokio` v1 - Async runtime with full features
- `tokio-postgres` v0.7 - PostgreSQL client with native protocol implementation
- `anyhow` v1.0 - Error handling and context
- `chrono` v0.4 - Timestamp handling for config table
- `postgres-types` v0.2 - PostgreSQL type support with chrono integration

#### Architecture
- **Modular Design**:
  - `src/cli.rs` - Command-line interface definitions
  - `src/db.rs` - Database connection management
  - `src/sql_templates.rs` - SQL template rendering and identifier quoting
  - `src/commands/` - Individual command implementations
  - `src/main.rs` - Application entry point and command dispatch

#### Commands

##### `init`
```bash
pg-app-role-manager init --database <DATABASE> --schema <SCHEMA> --role <ROLE>
```
Initializes the complete schema ownership pattern:
1. Creates database (if needed)
2. Creates schema (if needed)
3. Creates management role (if needed)
4. Configures schema ownership and permissions
5. Installs event trigger system
6. Adds initial schema-to-role mapping

##### `add-mapping`
```bash
pg-app-role-manager add-mapping --schema <SCHEMA> --role <ROLE>
```
Adds or updates a schema-to-role ownership mapping. Validates that both schema and role exist before creating the mapping.

##### `list-mappings`
```bash
pg-app-role-manager list-mappings
```
Displays all configured schema ownership mappings from the `schema_ownership_config` table.

##### `remove-mapping`
```bash
pg-app-role-manager remove-mapping --schema <SCHEMA>
```
Removes a schema-to-role ownership mapping. Reports the number of rows affected.

#### Global Connection Flags
All commands support:
- `--host <HOST>` - PostgreSQL host (env: `PGHOST`, default: localhost)
- `--port <PORT>` - PostgreSQL port (env: `PGPORT`, default: 5432)
- `--user <USER>` - PostgreSQL user (env: `PGUSER`, required)
- `--password <PASSWORD>` - PostgreSQL password (env: `PGPASSWORD`, required)
- `--dbname <DBNAME>` - Target database (env: `PGDATABASE`, context-dependent)

### Design Decisions

#### Out of Scope
- **User Grants**: Does NOT implement `GRANT role TO user` - administrators must handle this manually
- **Global Configuration**: Configuration is per-database, not stored in a global location
- **Dry-Run Mode**: No `--dry-run` flag in initial release
- **SSL/TLS Options**: No SSL connection configuration in initial release
- **Interactive Prompts**: No interactive password prompting
- **Automated Tests**: No unit or integration tests in initial release

#### Idempotency Strategy
- Database/schema/role creation: Check existence before creating
- Event trigger: Check `pg_event_trigger` before creating
- Config table: Uses `CREATE TABLE IF NOT EXISTS`
- Trigger function: Uses `CREATE OR REPLACE FUNCTION`
- Initial mapping: Uses `ON CONFLICT DO UPDATE`

### Security Considerations
- Passwords are passed via command-line flags or environment variables
- SQL injection prevention through identifier quoting and parameterized queries
- Event trigger function runs with `SECURITY DEFINER` to enable ownership transfers
- No privilege escalation beyond what's necessary for schema ownership management

### Known Limitations
- Requires PostgreSQL superuser or role with appropriate privileges to create databases, roles, and event triggers
- Event trigger only fires on DDL commands (not for objects created via dumps/restores)
- Password handling via CLI flags may expose credentials in process lists (use environment variables in production)

[unreleased]: https://github.com/yourusername/pg-app-role-manager/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/yourusername/pg-app-role-manager/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yourusername/pg-app-role-manager/releases/tag/v0.1.0
