pub struct SqlTemplates {
    pub database: String,
    pub schema: String,
    pub role: String,
}

impl SqlTemplates {
    pub fn new(database: String, schema: String, role: String) -> Self {
        Self {
            database,
            schema,
            role,
        }
    }

    pub fn create_database(&self) -> String {
        format!("CREATE DATABASE {}", self.quote_identifier(&self.database))
    }

    pub fn create_schema(&self) -> String {
        format!("CREATE SCHEMA {}", self.quote_identifier(&self.schema))
    }

    pub fn create_role(&self) -> String {
        format!("CREATE ROLE {} NOLOGIN", self.quote_identifier(&self.role))
    }

    pub fn grant_connect(&self) -> String {
        format!(
            "GRANT CONNECT ON DATABASE {} TO {}",
            self.quote_identifier(&self.database),
            self.quote_identifier(&self.role)
        )
    }

    pub fn alter_schema_owner(&self) -> String {
        format!(
            "ALTER SCHEMA {} OWNER TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn grant_schema_usage(&self) -> String {
        format!(
            "GRANT USAGE ON SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn grant_schema_create(&self) -> String {
        format!(
            "GRANT CREATE ON SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn grant_all_tables(&self) -> String {
        format!(
            "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn grant_all_sequences(&self) -> String {
        format!(
            "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn grant_all_functions(&self) -> String {
        format!(
            "GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn alter_default_privileges_tables(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL PRIVILEGES ON TABLES TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn alter_default_privileges_sequences(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL PRIVILEGES ON SEQUENCES TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    pub fn alter_default_privileges_functions(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL PRIVILEGES ON FUNCTIONS TO {}",
            self.quote_identifier(&self.schema),
            self.quote_identifier(&self.role)
        )
    }

    fn readonly_role(&self) -> String {
        crate::utils::quote_identifier(&format!("{}_ro", self.schema))
    }

    pub fn create_readonly_role(&self) -> String {
        format!("CREATE ROLE {} NOLOGIN", self.readonly_role())
    }

    pub fn grant_connect_readonly(&self) -> String {
        format!(
            "GRANT CONNECT ON DATABASE {} TO {}",
            self.quote_identifier(&self.database),
            self.readonly_role()
        )
    }

    pub fn grant_schema_usage_readonly(&self) -> String {
        format!(
            "GRANT USAGE ON SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn grant_select_tables(&self) -> String {
        format!(
            "GRANT SELECT ON ALL TABLES IN SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn grant_select_sequences(&self) -> String {
        format!(
            "GRANT SELECT ON ALL SEQUENCES IN SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn alter_default_privileges_select_tables(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} GRANT SELECT ON TABLES TO {}",
            self.quote_identifier(&self.role),
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn alter_default_privileges_select_sequences(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} GRANT SELECT ON SEQUENCES TO {}",
            self.quote_identifier(&self.role),
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn grant_execute_functions(&self) -> String {
        format!(
            "GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA {} TO {}",
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn alter_default_privileges_execute_functions(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} GRANT EXECUTE ON FUNCTIONS TO {}",
            self.quote_identifier(&self.role),
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn alter_default_privileges_usage_types(&self) -> String {
        format!(
            "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} GRANT USAGE ON TYPES TO {}",
            self.quote_identifier(&self.role),
            self.quote_identifier(&self.schema),
            self.readonly_role()
        )
    }

    pub fn create_config_table(&self) -> &'static str {
        r#"CREATE TABLE IF NOT EXISTS public.schema_ownership_config (
    schema_name name PRIMARY KEY,
    target_role name NOT NULL,
    created_at timestamptz DEFAULT now(),
    updated_at timestamptz DEFAULT now()
)"#
    }

    pub fn create_trigger_function(&self) -> &'static str {
        r#"CREATE OR REPLACE FUNCTION auto_transfer_schema_ownership()
RETURNS event_trigger
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    obj record;
    target_role_name name;
    target_role_oid oid;
    current_owner_oid oid;
BEGIN
    FOR obj IN SELECT * FROM pg_event_trigger_ddl_commands()
    LOOP
        SELECT target_role INTO target_role_name
        FROM public.schema_ownership_config
        WHERE schema_name = obj.schema_name;

        IF target_role_name IS NOT NULL THEN
            SELECT oid INTO target_role_oid
            FROM pg_roles
            WHERE rolname = target_role_name;

            IF target_role_oid IS NULL THEN
                CONTINUE;
            END IF;

            current_owner_oid := NULL;

            CASE obj.object_type
                WHEN 'table', 'sequence', 'view', 'materialized view' THEN
                    SELECT relowner INTO current_owner_oid
                    FROM pg_class
                    WHERE oid = obj.objid;

                WHEN 'function' THEN
                    SELECT proowner INTO current_owner_oid
                    FROM pg_proc
                    WHERE oid = obj.objid;

                WHEN 'type' THEN
                    SELECT typowner INTO current_owner_oid
                    FROM pg_type
                    WHERE oid = obj.objid;

                ELSE
                    -- Ignore other object types (index, trigger, etc.)
                    NULL;
            END CASE;

            IF current_owner_oid IS NOT NULL AND current_owner_oid != target_role_oid THEN
                CASE obj.object_type
                    WHEN 'table' THEN
                        EXECUTE format('ALTER TABLE %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
                    WHEN 'sequence' THEN
                        -- Skip sequences owned by table columns (created by SERIAL/BIGSERIAL/IDENTITY)
                        -- ALTER TABLE automatically transfers ownership of dependent sequences
                        -- deptype 'a' = auto, 'i' = internal (both indicate column ownership)
                        IF NOT EXISTS (
                            SELECT 1 FROM pg_depend
                            WHERE objid = obj.objid
                              AND deptype IN ('a', 'i')
                              AND classid = 'pg_class'::regclass
                              AND refclassid = 'pg_class'::regclass
                        ) THEN
                            EXECUTE format('ALTER SEQUENCE %s OWNER TO %I',
                                         obj.object_identity, target_role_name);
                        END IF;
                    WHEN 'view' THEN
                        EXECUTE format('ALTER VIEW %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
                    WHEN 'materialized view' THEN
                        EXECUTE format('ALTER MATERIALIZED VIEW %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
                    WHEN 'function' THEN
                        EXECUTE format('ALTER FUNCTION %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
                    WHEN 'type' THEN
                        EXECUTE format('ALTER TYPE %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
                    ELSE
                        -- Ignore other object types (index, trigger, etc.)
                        NULL;
                END CASE;
            END IF;
        END IF;
    END LOOP;
END;
$$"#
    }

    pub fn create_event_trigger(&self) -> &'static str {
        r#"CREATE EVENT TRIGGER auto_transfer_schema_ownership_trigger
ON ddl_command_end
EXECUTE FUNCTION auto_transfer_schema_ownership()"#
    }

    pub fn insert_initial_mapping(&self) -> String {
        format!(
            "INSERT INTO public.schema_ownership_config (schema_name, target_role) VALUES ('{}', '{}') ON CONFLICT (schema_name) DO NOTHING",
            self.schema, self.role
        )
    }

    pub fn alter_database_search_path(&self) -> String {
        format!(
            "ALTER DATABASE {} SET search_path TO {}",
            self.quote_identifier(&self.database),
            self.quote_identifier(&self.schema)
        )
    }

    fn quote_identifier(&self, name: &str) -> String {
        crate::utils::quote_identifier(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make() -> SqlTemplates {
        SqlTemplates::new("mydb".into(), "myschema".into(), "myrole".into())
    }

    // ── Individual SQL methods ───────────────────────────────────────────────

    #[test]
    fn create_database() {
        assert_eq!(make().create_database(), "CREATE DATABASE \"mydb\"");
    }

    #[test]
    fn create_schema() {
        assert_eq!(make().create_schema(), "CREATE SCHEMA \"myschema\"");
    }

    #[test]
    fn create_role() {
        assert_eq!(make().create_role(), "CREATE ROLE \"myrole\" NOLOGIN");
    }

    #[test]
    fn grant_connect() {
        let sql = make().grant_connect();
        assert!(sql.contains("GRANT CONNECT ON DATABASE"));
        assert!(sql.contains("\"mydb\""));
        assert!(sql.contains("\"myrole\""));
    }

    #[test]
    fn alter_schema_owner() {
        assert_eq!(
            make().alter_schema_owner(),
            "ALTER SCHEMA \"myschema\" OWNER TO \"myrole\""
        );
    }

    #[test]
    fn grant_schema_usage() {
        let sql = make().grant_schema_usage();
        assert!(sql.contains("GRANT USAGE ON SCHEMA"));
        assert!(sql.contains("\"myschema\""));
        assert!(sql.contains("\"myrole\""));
    }

    #[test]
    fn grant_schema_create() {
        let sql = make().grant_schema_create();
        assert!(sql.contains("GRANT CREATE ON SCHEMA"));
    }

    #[test]
    fn grant_all_tables() {
        let sql = make().grant_all_tables();
        assert!(sql.contains("GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA"));
        assert!(sql.contains("\"myschema\""));
        assert!(sql.contains("\"myrole\""));
    }

    #[test]
    fn grant_all_sequences() {
        assert!(make().grant_all_sequences().contains("ALL SEQUENCES IN SCHEMA"));
    }

    #[test]
    fn grant_all_functions() {
        assert!(make().grant_all_functions().contains("ALL FUNCTIONS IN SCHEMA"));
    }

    #[test]
    fn alter_default_privileges_tables() {
        let sql = make().alter_default_privileges_tables();
        assert!(sql.contains("ALTER DEFAULT PRIVILEGES IN SCHEMA"));
        assert!(sql.contains("ON TABLES TO"));
    }

    #[test]
    fn alter_default_privileges_sequences() {
        assert!(make().alter_default_privileges_sequences().contains("ON SEQUENCES TO"));
    }

    #[test]
    fn alter_default_privileges_functions() {
        assert!(make().alter_default_privileges_functions().contains("ON FUNCTIONS TO"));
    }

    #[test]
    fn create_config_table_contains_expected_columns() {
        let sql = make().create_config_table();
        assert!(sql.contains("schema_ownership_config"));
        assert!(sql.contains("schema_name"));
        assert!(sql.contains("target_role"));
        assert!(sql.contains("created_at"));
        assert!(sql.contains("updated_at"));
    }

    #[test]
    fn create_trigger_function_contains_key_elements() {
        let sql = make().create_trigger_function();
        assert!(sql.contains("auto_transfer_schema_ownership"));
        assert!(sql.contains("SECURITY DEFINER"));
        assert!(sql.contains("pg_event_trigger_ddl_commands"));
        // All handled object types must appear
        for kind in &["table", "sequence", "view", "materialized view", "function", "type"] {
            assert!(sql.contains(kind), "trigger function missing object type '{}'", kind);
        }
    }

    #[test]
    fn create_event_trigger_contains_key_elements() {
        let sql = make().create_event_trigger();
        assert!(sql.contains("auto_transfer_schema_ownership_trigger"));
        assert!(sql.contains("ddl_command_end"));
    }

    #[test]
    fn insert_initial_mapping_contains_values() {
        let sql = make().insert_initial_mapping();
        assert!(sql.contains("schema_ownership_config"));
        assert!(sql.contains("'myschema'"));
        assert!(sql.contains("'myrole'"));
        assert!(sql.contains("ON CONFLICT"));
    }

    // ── Injection prevention via embedded double quotes ──────────────────────

    #[test]
    fn schema_name_with_embedded_quote_is_escaped() {
        // Schema name:  bad"schema
        // Expected SQL: CREATE SCHEMA "bad""schema"
        let t = SqlTemplates::new("db".into(), "bad\"schema".into(), "role".into());
        assert_eq!(t.create_schema(), "CREATE SCHEMA \"bad\"\"schema\"");
    }

    #[test]
    fn role_name_with_embedded_quote_is_escaped() {
        let t = SqlTemplates::new("db".into(), "schema".into(), "bad\"role".into());
        assert_eq!(t.create_role(), "CREATE ROLE \"bad\"\"role\" NOLOGIN");
    }

    #[test]
    fn database_name_with_embedded_quote_is_escaped() {
        let t = SqlTemplates::new("bad\"db".into(), "schema".into(), "role".into());
        assert_eq!(t.create_database(), "CREATE DATABASE \"bad\"\"db\"");
    }

    // ── Read-only role methods ───────────────────────────────────────────────

    #[test]
    fn create_readonly_role_has_ro_suffix() {
        let sql = make().create_readonly_role();
        assert_eq!(sql, "CREATE ROLE \"myschema_ro\" NOLOGIN");
    }

    #[test]
    fn grant_connect_readonly_targets_ro_role() {
        let sql = make().grant_connect_readonly();
        assert!(sql.contains("GRANT CONNECT ON DATABASE"));
        assert!(sql.contains("\"mydb\""));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("\"myrole\""));
    }

    #[test]
    fn grant_schema_usage_readonly_targets_ro_role() {
        let sql = make().grant_schema_usage_readonly();
        assert!(sql.contains("GRANT USAGE ON SCHEMA"));
        assert!(sql.contains("\"myschema\""));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("\"myrole\""));
    }

    #[test]
    fn grant_select_tables_is_select_only() {
        let sql = make().grant_select_tables();
        assert!(sql.contains("GRANT SELECT ON ALL TABLES IN SCHEMA"));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("ALL PRIVILEGES"));
        assert!(!sql.contains("INSERT"));
        assert!(!sql.contains("UPDATE"));
        assert!(!sql.contains("DELETE"));
    }

    #[test]
    fn grant_select_sequences_is_select_only() {
        let sql = make().grant_select_sequences();
        assert!(sql.contains("GRANT SELECT ON ALL SEQUENCES IN SCHEMA"));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("ALL PRIVILEGES"));
    }

    #[test]
    fn alter_default_privileges_select_tables_is_select_only() {
        let sql = make().alter_default_privileges_select_tables();
        assert!(sql.contains("ALTER DEFAULT PRIVILEGES FOR ROLE \"myrole\""));
        assert!(sql.contains("IN SCHEMA"));
        assert!(sql.contains("GRANT SELECT ON TABLES TO"));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("ALL PRIVILEGES"));
    }

    #[test]
    fn alter_default_privileges_select_sequences_is_select_only() {
        let sql = make().alter_default_privileges_select_sequences();
        assert!(sql.contains("ALTER DEFAULT PRIVILEGES FOR ROLE \"myrole\""));
        assert!(sql.contains("IN SCHEMA"));
        assert!(sql.contains("GRANT SELECT ON SEQUENCES TO"));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("ALL PRIVILEGES"));
    }

    #[test]
    fn grant_execute_functions_targets_ro_role() {
        let sql = make().grant_execute_functions();
        assert!(sql.contains("GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA"));
        assert!(sql.contains("\"myschema\""));
        assert!(sql.contains("\"myschema_ro\""));
        assert!(!sql.contains("\"myrole\""));
    }

    #[test]
    fn alter_default_privileges_execute_functions_targets_ro_role() {
        let sql = make().alter_default_privileges_execute_functions();
        assert!(sql.contains("ALTER DEFAULT PRIVILEGES FOR ROLE \"myrole\""));
        assert!(sql.contains("GRANT EXECUTE ON FUNCTIONS TO"));
        assert!(sql.contains("\"myschema_ro\""));
    }

    #[test]
    fn alter_default_privileges_usage_types_targets_ro_role() {
        let sql = make().alter_default_privileges_usage_types();
        assert!(sql.contains("ALTER DEFAULT PRIVILEGES FOR ROLE \"myrole\""));
        assert!(sql.contains("GRANT USAGE ON TYPES TO"));
        assert!(sql.contains("\"myschema_ro\""));
    }

    #[test]
    fn readonly_role_name_escapes_schema_quote() {
        let t = SqlTemplates::new("db".into(), "bad\"schema".into(), "role".into());
        assert_eq!(t.create_readonly_role(), "CREATE ROLE \"bad\"\"schema_ro\" NOLOGIN");
    }

    #[test]
    fn alter_database_search_path_contains_both_identifiers() {
        let sql = make().alter_database_search_path();
        assert_eq!(sql, "ALTER DATABASE \"mydb\" SET search_path TO \"myschema\"");
    }
}
