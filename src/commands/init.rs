use anyhow::{Context, Result};
use tokio_postgres::Client;

use crate::db::{connect, ConnectionConfig};
use crate::report::{ActionOutcome, ActionReport};
use crate::sql_templates::SqlTemplates;

pub async fn execute(conn_opts: ConnectionConfig, database: String, schema: String, role: String, verbose: u8) -> Result<()> {
    let mut report = ActionReport::new("Init");
    let templates = SqlTemplates::new(database.clone(), schema.clone(), role.clone());

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
    if schema_exists(&client, &schema, verbose).await? {
        report.record(format!("Schema '{}'", schema), ActionOutcome::Skipped);
    } else {
        let sql = templates.create_schema();
        log_sql(&sql, 1);
        client.execute(&sql, &[]).await
            .context("Failed to create schema")?;
        report.record(format!("Schema '{}'", schema), ActionOutcome::Created);
    }

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
        client.execute(sql, &[]).await
            .context("Failed to create event trigger")?;
        report.record("Event trigger", ActionOutcome::Created);
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
