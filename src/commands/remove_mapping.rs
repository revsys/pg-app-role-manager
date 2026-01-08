use anyhow::{Context, Result};

use crate::db::{connect, ConnectionConfig};
use crate::report::{ActionOutcome, ActionReport};

pub async fn execute(conn_opts: ConnectionConfig, schema: String, verbose: u8) -> Result<()> {
    let mut report = ActionReport::new("Remove Mapping");
    let client = connect(&conn_opts).await?;

    let sql = "DELETE FROM public.schema_ownership_config WHERE schema_name = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, schema);
    }
    let rows_affected = match client
        .execute(sql, &[&schema])
        .await
    {
        Ok(count) => count,
        Err(e) => {
            // Check if the error is because the table doesn't exist
            if let Some(db_err) = e.as_db_error() {
                if db_err.code().code() == "42P01" {
                    // SQLSTATE 42P01: undefined_table
                    return Err(anyhow::anyhow!("Schema ownership pattern not initialized in this database. Run 'init' command first."));
                }
            }
            return Err(e).context("Failed to delete mapping");
        }
    };

    if rows_affected == 0 {
        report.record(
            format!("Mapping for schema '{}'", schema),
            ActionOutcome::NotFound,
        );
    } else {
        report.record(
            format!("Mapping for schema '{}'", schema),
            ActionOutcome::Removed,
        );
    }

    report.print_summary();

    Ok(())
}
