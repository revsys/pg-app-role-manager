-- Create role for managing database objects in _foundation_ database for _foundation_ schema

-- Create the role (choose one approach)

create database foundation;

-- Option 1: Create a role that can login
CREATE ROLE foundation_manager NOLOGIN;

GRANT foundation_manager to stephen;
GRANT CONNECT on database foundation to foundation_manager;


-- Option 2: Create a role without login (for use with GRANT)
-- CREATE ROLE foundation_manager;

-- Connect to the _foundation_ database
\c foundation;

CREATE SCHEMA app;

alter database foundation set search_path to app;

-- Set the schema owner to foundation_manager so objects created in it are owned by this role
ALTER SCHEMA app OWNER TO foundation_manager;

-- Grant USAGE on the schema (required to access objects in it)
GRANT USAGE ON SCHEMA app TO foundation_manager;

-- Grant all privileges on the schema (allows creating objects)
GRANT CREATE ON SCHEMA app TO foundation_manager;

-- Grant all privileges on existing tables in the schema
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA app TO foundation_manager;

-- Grant all privileges on existing sequences
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA app TO foundation_manager;

-- Grant all privileges on existing functions
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA app TO foundation_manager;

-- Grant privileges on future objects (PostgreSQL 9.0+)
ALTER DEFAULT PRIVILEGES IN SCHEMA app
    GRANT ALL PRIVILEGES ON TABLES TO foundation_manager;

ALTER DEFAULT PRIVILEGES IN SCHEMA app
    GRANT ALL PRIVILEGES ON SEQUENCES TO foundation_manager;

ALTER DEFAULT PRIVILEGES IN SCHEMA app
    GRANT ALL PRIVILEGES ON FUNCTIONS TO foundation_manager;

-- Optional: Grant ability to create other roles and grant permissions
-- GRANT app TO foundation_manager WITH ADMIN OPTION;

-- Additional attributes you might want to add to the role:
-- ALTER ROLE foundation_manager CREATEDB;      -- can create databases
-- ALTER ROLE foundation_manager CREATEROLE;    -- can create other roles
-- ALTER ROLE foundation_manager SUPERUSER;     -- full privileges (use cautiously)


-- Configuration table to store target role for each schema
CREATE TABLE IF NOT EXISTS public.schema_ownership_config (
    schema_name name PRIMARY KEY,
    target_role name NOT NULL,
    created_at timestamptz DEFAULT now(),
    updated_at timestamptz DEFAULT now()
);

-- Insert configuration for the app schema
INSERT INTO public.schema_ownership_config (schema_name, target_role)
VALUES ('app', 'foundation_manager')
ON CONFLICT (schema_name) DO UPDATE SET target_role = EXCLUDED.target_role, updated_at = now();

-- Event trigger function to automatically transfer ownership of objects created in configured schemas
-- Short-circuits if object is already owned by the target role
CREATE OR REPLACE FUNCTION auto_transfer_schema_ownership()
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
    -- Loop through all objects created by this DDL command
    FOR obj IN SELECT * FROM pg_event_trigger_ddl_commands()
    LOOP
        -- Check if this schema has a configured target role
        SELECT target_role INTO target_role_name
        FROM public.schema_ownership_config
        WHERE schema_name = obj.schema_name;

        -- Only process if we found a configuration for this schema
        IF target_role_name IS NOT NULL THEN
            -- Get the OID of the target role
            SELECT oid INTO target_role_oid
            FROM pg_roles
            WHERE rolname = target_role_name;

            -- Skip if role doesn't exist
            IF target_role_oid IS NULL THEN
                CONTINUE;
            END IF;

            -- Get current owner OID based on object type
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

            -- Only transfer ownership if it's not already owned by target role
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
$$;

-- Create event trigger that fires after any DDL command
CREATE EVENT TRIGGER auto_transfer_schema_ownership_trigger
ON ddl_command_end
EXECUTE FUNCTION auto_transfer_schema_ownership();


create table app.things ( first varchar(32));


-- To use this ownership transfer system with other schemas/databases:
-- 1. Create the public.schema_ownership_config table if it doesn't exist
-- 2. Insert a row mapping your schema name to the target role:
--    INSERT INTO public.schema_ownership_config (schema_name, target_role)
--    VALUES ('your_schema_name', 'your_target_role');
-- 3. The event trigger will automatically transfer ownership for all configured schemas

