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
            END CASE;

            IF current_owner_oid IS NOT NULL AND current_owner_oid != target_role_oid THEN
                CASE obj.object_type
                    WHEN 'table' THEN
                        EXECUTE format('ALTER TABLE %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
                    WHEN 'sequence' THEN
                        EXECUTE format('ALTER SEQUENCE %s OWNER TO %I',
                                     obj.object_identity, target_role_name);
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

    fn quote_identifier(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}
