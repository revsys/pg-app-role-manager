# Rust CLI for PostgreSQL Schema Ownership Pattern - Remaining Tasks

## Project Decisions
- **Binary name**: `pg-app-role-manager`
- **Location**: Current directory (user-config/)
- **Dependencies**: All pure Rust (no external C libraries required)
- **Scope**: Per-database (config table and triggers in each database, not global)
- **Idempotency**: Skip and continue if objects exist
- **User grants**: NOT implemented (admins handle `GRANT role TO user` manually)

---

## Pending Work

### Future Enhancements (Not in Current Scope)
- [ ] Dry-run mode (--dry-run flag)
- [ ] Colored output
- [ ] Interactive password prompts
- [ ] SSL/TLS connection options
- [ ] Unit and integration tests
- [ ] Terminal width detection for truly dynamic pagination

---

See ARCHIVE.md for all completed tasks.
