use anyhow::{Context, Result};
use tokio_postgres::{Client, error::SqlState};

use crate::db::{connect, ConnectionConfig};
use crate::report::{ActionOutcome, ActionReport};
use crate::sql_templates::SqlTemplates;

pub async fn execute(conn_opts: ConnectionConfig, database: String, schema: String, role: String, ro_role: String, verbose: u8) -> Result<()> {
    // Block operations on system databases (PostgreSQL + cloud providers)
    let blocked_databases = ["postgres", "template0", "template1", "rdsadmin", "azure_maintenance", "cloudsqladmin"];
    if blocked_databases.contains(&database.as_str()) {
        anyhow::bail!(
            "Cannot initialize schema ownership management on system database '{}'. \
             System databases (postgres, template0, template1, rdsadmin, etc.) are reserved for internal use.",
            database
        );
    }

    let mut report = ActionReport::new("Init");
    let templates = SqlTemplates::new(database.clone(), schema.clone(), role.clone(), ro_role.clone());

    // Helper to print SQL in verbose mode
    let log_sql = |sql: &str, min_level: u8| {
        if verbose >= min_level {
            println!("[SQL] {}", sql);
        }
    };

    // Connect to postgres system database
    let mut config = conn_opts.clone();
    config.dbname = Some("postgres".to_string());
    let client = connect(&config).await?;

    // Check and create database
    if database_exists(&client, &database, verbose).await? {
        report.record(format!("Database '{}'", database), ActionOutcome::Skipped);
    } else {
        let sql = templates.create_database();
        log_sql(&sql, 1);
        client.execute(&sql, &[]).await
            .context("Failed to create database")?;
        report.record(format!("Database '{}'", database), ActionOutcome::Created);
    }

    // Reconnect to target database
    drop(client);
    let mut target_config = conn_opts.clone();
    target_config.dbname = Some(database.clone());
    let client = connect(&target_config).await?;

    // Check and create schema
    let schema_already_exists = schema_exists(&client, &schema, verbose).await?;
    if schema_already_exists {
        // Schema exists - check if there's already a mapping for it
        if let Some(existing_role) = get_schema_mapping(&client, &schema, verbose).await? {
            if existing_role != role {
                anyhow::bail!(
                    "Schema '{}' is already mapped to role '{}'. Schema-to-role mappings are immutable after initialization. \
                     To change the mapping, you must manually update the database using SQL.",
                    schema, existing_role
                );
            }
            // else: Same role, continue idempotently
        }
        report.record(format!("Schema '{}'", schema), ActionOutcome::Skipped);
    } else {
        let sql = templates.create_schema();
        log_sql(&sql, 1);
        client.execute(&sql, &[]).await
            .context("Failed to create schema")?;
        report.record(format!("Schema '{}'", schema), ActionOutcome::Created);
    }

    // Set database-level default search_path to the app schema
    let sql = templates.alter_database_search_path();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to set database search_path")?;
    report.record("Database search_path", ActionOutcome::Updated);

    // Check and create role
    if role_exists(&client, &role, verbose).await? {
        report.record(format!("Role '{}'", role), ActionOutcome::Skipped);
    } else {
        let sql = templates.create_role();
        log_sql(&sql, 1);
        client.execute(&sql, &[]).await
            .context("Failed to create role")?;
        report.record(format!("Role '{}'", role), ActionOutcome::Created);
    }

    // Check and create read-only role
    if role_exists(&client, &ro_role, verbose).await? {
        report.record(format!("Role '{}'", ro_role), ActionOutcome::Skipped);
    } else {
        let sql = templates.create_readonly_role();
        log_sql(&sql, 1);
        client.execute(&sql, &[]).await
            .context("Failed to create read-only role")?;
        report.record(format!("Role '{}'", ro_role), ActionOutcome::Created);
    }

    let sql = templates.grant_connect_readonly();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant CONNECT to read-only role")?;
    report.record(format!("CONNECT privilege ({})", ro_role), ActionOutcome::Updated);

    let sql = templates.grant_schema_usage_readonly();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant USAGE on schema to read-only role")?;
    report.record(format!("USAGE on schema ({})", ro_role), ActionOutcome::Updated);

    let sql = templates.grant_select_tables();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant SELECT on tables to read-only role")?;
    report.record(format!("SELECT on tables ({})", ro_role), ActionOutcome::Updated);

    let sql = templates.grant_select_sequences();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant SELECT on sequences to read-only role")?;
    report.record(format!("SELECT on sequences ({})", ro_role), ActionOutcome::Updated);

    // SET ROLE so the default privilege grants below apply to objects created by the app role.
    // We first grant the app role to the current user so SET ROLE works on managed PostgreSQL
    // (RDS, Cloud SQL, etc.) where the master user is not a true superuser.
    let sql = templates.grant_role_to_current_user();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant app role to current user")?;
    let sql = templates.set_role();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to SET ROLE to app role")?;

    let sql = templates.alter_default_privileges_select_tables();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to set default SELECT privileges on tables for read-only role")?;
    report.record(format!("Default SELECT on tables ({})", ro_role), ActionOutcome::Updated);

    let sql = templates.alter_default_privileges_select_sequences();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to set default SELECT privileges on sequences for read-only role")?;
    report.record(format!("Default SELECT on sequences ({})", ro_role), ActionOutcome::Updated);

    let sql = templates.alter_default_privileges_execute_functions();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to set default EXECUTE privileges on functions for read-only role")?;
    report.record(format!("Default EXECUTE on functions ({})", ro_role), ActionOutcome::Updated);

    let sql = templates.alter_default_privileges_usage_types();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to set default USAGE privileges on types for read-only role")?;
    report.record(format!("Default USAGE on types ({})", ro_role), ActionOutcome::Updated);

    client.execute("RESET ROLE", &[]).await
        .context("Failed to RESET ROLE after setting default privileges")?;

    let sql = templates.grant_execute_functions();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant EXECUTE on functions to read-only role")?;
    report.record(format!("EXECUTE on functions ({})", ro_role), ActionOutcome::Updated);

    // Set up schema ownership and grants
    let sql = templates.grant_connect();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant CONNECT")?;
    report.record("CONNECT privilege", ActionOutcome::Updated);

    let sql = templates.alter_schema_owner();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to alter schema owner")?;
    report.record("Schema ownership", ActionOutcome::Updated);

    let sql = templates.grant_schema_usage();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant USAGE on schema")?;
    report.record("USAGE on schema", ActionOutcome::Updated);

    let sql = templates.grant_schema_create();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant CREATE on schema")?;
    report.record("CREATE on schema", ActionOutcome::Updated);

    let sql = templates.grant_all_tables();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant privileges on tables")?;
    report.record("ALL on tables", ActionOutcome::Updated);

    let sql = templates.grant_all_sequences();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant privileges on sequences")?;
    report.record("ALL on sequences", ActionOutcome::Updated);

    let sql = templates.grant_all_functions();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to grant privileges on functions")?;
    report.record("ALL on functions", ActionOutcome::Updated);

    let sql = templates.alter_default_privileges_tables();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to alter default privileges for tables")?;
    report.record("Default privileges for tables", ActionOutcome::Updated);

    let sql = templates.alter_default_privileges_sequences();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to alter default privileges for sequences")?;
    report.record("Default privileges for sequences", ActionOutcome::Updated);

    let sql = templates.alter_default_privileges_functions();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to alter default privileges for functions")?;
    report.record("Default privileges for functions", ActionOutcome::Updated);

    // Create config table
    let sql = templates.create_config_table();
    log_sql(sql, 1);
    client.execute(sql, &[]).await
        .context("Failed to create config table")?;
    report.record("Config table", ActionOutcome::Created);

    // Create trigger function (only log at verbosity level 2+)
    let sql = templates.create_trigger_function();
    log_sql(sql, 2);
    client.execute(sql, &[]).await
        .context("Failed to create trigger function")?;
    report.record("Trigger function", ActionOutcome::Updated);

    // Create event trigger if it doesn't exist
    if event_trigger_exists(&client, "auto_transfer_schema_ownership_trigger", verbose).await? {
        report.record("Event trigger", ActionOutcome::Skipped);
    } else {
        let sql = templates.create_event_trigger();
        log_sql(sql, 1);
        match client.execute(sql, &[]).await {
            Ok(_) => {
                report.record("Event trigger", ActionOutcome::Created);
            }
            Err(ref e) if e.code() == Some(&SqlState::INSUFFICIENT_PRIVILEGE) => {
                eprintln!(
                    "Warning: Could not create event trigger (insufficient privileges). \
                     Managed PostgreSQL services (DigitalOcean, etc.) require superuser for \
                     event triggers. Automatic ownership transfer is disabled; new objects \
                     must be manually reassigned to role '{}'.",
                    role
                );
                report.record("Event trigger", ActionOutcome::Skipped);
            }
            Err(e) => {
                return Err(anyhow::anyhow!(e).context("Failed to create event trigger"));
            }
        }
    }

    // Insert initial mapping
    let sql = templates.insert_initial_mapping();
    log_sql(&sql, 1);
    client.execute(&sql, &[]).await
        .context("Failed to insert initial mapping")?;
    report.record("Initial mapping", ActionOutcome::Updated);

    report.print_summary();

    Ok(())
}

async fn database_exists(client: &Client, database: &str, verbose: u8) -> Result<bool> {
    let sql = "SELECT 1 FROM pg_database WHERE datname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, database);
    }
    let row = client
        .query_one(sql, &[&database])
        .await;
    Ok(row.is_ok())
}

async fn schema_exists(client: &Client, schema: &str, verbose: u8) -> Result<bool> {
    let sql = "SELECT 1 FROM pg_namespace WHERE nspname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, schema);
    }
    let row = client
        .query_one(sql, &[&schema])
        .await;
    Ok(row.is_ok())
}

async fn role_exists(client: &Client, role: &str, verbose: u8) -> Result<bool> {
    let sql = "SELECT 1 FROM pg_roles WHERE rolname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, role);
    }
    let row = client
        .query_one(sql, &[&role])
        .await;
    Ok(row.is_ok())
}

async fn event_trigger_exists(client: &Client, trigger_name: &str, verbose: u8) -> Result<bool> {
    let sql = "SELECT 1 FROM pg_event_trigger WHERE evtname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, trigger_name);
    }
    let row = client
        .query_one(sql, &[&trigger_name])
        .await;
    Ok(row.is_ok())
}

async fn get_schema_mapping(client: &Client, schema: &str, verbose: u8) -> Result<Option<String>> {
    let sql = "SELECT target_role FROM public.schema_ownership_config WHERE schema_name = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, schema);
    }
    match client.query_one(sql, &[&schema]).await {
        Ok(row) => {
            let role: String = row.get(0);
            Ok(Some(role))
        }
        Err(_) => Ok(None),
    }
}
