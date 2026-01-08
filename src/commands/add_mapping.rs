use anyhow::{anyhow, Context, Result};

use crate::db::{connect, ConnectionConfig};
use crate::report::{ActionOutcome, ActionReport};

pub async fn execute(conn_opts: ConnectionConfig, schema: String, role: String, verbose: u8) -> Result<()> {
    let mut report = ActionReport::new("Add Mapping");
    let client = connect(&conn_opts).await?;

    // Validate schema exists
    let sql = "SELECT 1 FROM pg_namespace WHERE nspname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, schema);
    }
    let schema_row = client
        .query_opt(sql, &[&schema])
        .await
        .context("Failed to query pg_namespace")?;

    if schema_row.is_none() {
        return Err(anyhow!("Schema '{}' does not exist", schema));
    }

    // Validate role exists
    let sql = "SELECT 1 FROM pg_roles WHERE rolname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, role);
    }
    let role_row = client
        .query_opt(sql, &[&role])
        .await
        .context("Failed to query pg_roles")?;

    if role_row.is_none() {
        return Err(anyhow!("Role '{}' does not exist", role));
    }

    // Insert or update mapping
    let sql = "INSERT INTO public.schema_ownership_config (schema_name, target_role) VALUES ($1, $2) ON CONFLICT (schema_name) DO UPDATE SET target_role = EXCLUDED.target_role, updated_at = now()";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}  ,  {}]", sql, schema, role);
    }
    if let Err(e) = client.execute(sql, &[&schema, &role]).await {
        // Check if the error is because the table doesn't exist
        if let Some(db_err) = e.as_db_error() {
            if db_err.code().code() == "42P01" {
                // SQLSTATE 42P01: undefined_table
                return Err(anyhow!("Schema ownership pattern not initialized in this database. Run 'init' command first."));
            }
        }
        return Err(e).context("Failed to insert mapping");
    }

    report.record(
        format!("Mapping: schema '{}' -> role '{}'", schema, role),
        ActionOutcome::Updated,
    );

    report.print_summary();

    Ok(())
}
