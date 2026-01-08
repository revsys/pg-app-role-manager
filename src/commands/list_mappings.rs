use anyhow::{Context, Result};

use crate::db::{connect, ConnectionConfig};

fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}[...]", &s[..max_len.saturating_sub(5)])
    }
}

pub async fn execute(conn_opts: ConnectionConfig, verbose: u8) -> Result<()> {
    let client = connect(&conn_opts).await?;

    let sql = "SELECT schema_name, target_role, created_at, updated_at FROM public.schema_ownership_config ORDER BY schema_name";
    if verbose >= 1 {
        println!("[SQL] {}", sql);
    }
    let rows = match client
        .query(sql, &[])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            // Check if the error is because the table doesn't exist
            if let Some(db_err) = e.as_db_error() {
                if db_err.code().code() == "42P01" {
                    // SQLSTATE 42P01: undefined_table
                    println!("Schema ownership pattern not initialized in this database.");
                    println!("Run 'init' command first to set up the pattern.");
                    return Ok(());
                }
            }
            return Err(e).context("Failed to query schema_ownership_config");
        }
    };

    if rows.is_empty() {
        println!("No schema-to-role mappings found.");
        return Ok(());
    }

    println!("{:<20} {:<44} {:<21} {:<19}", "Schema", "Target Role", "Created At", "Updated At");
    println!("{}", "-".repeat(108));

    for row in &rows {
        let schema_name: String = row.get(0);
        let target_role: String = row.get(1);
        let created_at: chrono::DateTime<chrono::Utc> = row.get(2);
        let updated_at: chrono::DateTime<chrono::Utc> = row.get(3);

        let truncated_role = truncate_with_ellipsis(&target_role, 30);

        println!(
            "{:<20} {:<44} {:<21} {:<19}",
            schema_name,
            truncated_role,
            created_at.format("%Y-%m-%d %H:%M:%S"),
            updated_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    println!();
    println!("Total mappings: {}", rows.len());

    Ok(())
}
