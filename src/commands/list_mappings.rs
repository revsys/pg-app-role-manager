use anyhow::{Context, Result};

use crate::db::{connect, ConnectionConfig};

fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}[...]", &s[..max_len.saturating_sub(5)])
    }
}

#[derive(Debug)]
struct MappingRow {
    database: String,
    schema_name: String,
    target_role: String,
    granted_to: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn execute(conn_opts: ConnectionConfig, verbose: u8) -> Result<()> {
    // Connect to postgres system database to get list of all databases
    let mut config = conn_opts.clone();
    config.dbname = Some("postgres".to_string());
    let client = connect(&config).await?;

    // Query for all non-system databases
    // Blocked: PostgreSQL core + cloud provider (AWS RDS, Azure, GCP) system databases
    let blocked_databases = ["postgres", "template0", "template1", "rdsadmin", "azure_maintenance", "cloudsqladmin"];
    let sql = "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname";
    if verbose >= 1 {
        println!("[SQL] {}", sql);
    }
    let db_rows = client.query(sql, &[])
        .await
        .context("Failed to query pg_database")?;

    let databases: Vec<String> = db_rows
        .iter()
        .map(|row| row.get(0))
        .filter(|dbname: &String| !blocked_databases.contains(&dbname.as_str()))
        .collect();

    if databases.is_empty() {
        println!("No non-system databases found.");
        return Ok(());
    }

    // Query each database for schema_ownership_config mappings
    let mut all_mappings = Vec::new();
    let mut databases_with_mappings: std::collections::HashSet<String> = std::collections::HashSet::new();

    for database in &databases {
        let mut db_config = conn_opts.clone();
        db_config.dbname = Some(database.clone());

        let db_client = match connect(&db_config).await {
            Ok(client) => client,
            Err(e) => {
                if verbose >= 1 {
                    println!("Warning: Failed to connect to database '{}': {}", database, e);
                }
                continue;
            }
        };

        let sql = "SELECT schema_name, target_role, created_at, updated_at FROM public.schema_ownership_config ORDER BY schema_name";
        if verbose >= 1 {
            println!("[SQL] {} (database: {})", sql, database);
        }

        let rows = match db_client.query(sql, &[]).await {
            Ok(rows) => rows,
            Err(e) => {
                // Check if the error is because the table doesn't exist
                if let Some(db_err) = e.as_db_error() {
                    if db_err.code().code() == "42P01" {
                        // SQLSTATE 42P01: undefined_table - skip this database
                        if verbose >= 1 {
                            println!("  No schema_ownership_config in database '{}'", database);
                        }
                        continue;
                    }
                }
                if verbose >= 1 {
                    println!("Warning: Failed to query database '{}': {}", database, e);
                }
                continue;
            }
        };

        if !rows.is_empty() {
            databases_with_mappings.insert(database.clone());
        }

        for row in rows {
            let target_role: String = row.get(1);

            // Query for roles/users that have been granted this target role
            let members_sql = "
                SELECT r.rolname
                FROM pg_roles r
                JOIN pg_auth_members m ON m.member = r.oid
                JOIN pg_roles g ON m.roleid = g.oid
                WHERE g.rolname = $1
                ORDER BY r.rolname
            ";

            if verbose >= 2 {
                println!("[SQL] {} (database: {}, role: {})", members_sql.trim(), database, target_role);
            }

            let granted_to = match db_client.query(members_sql, &[&target_role]).await {
                Ok(member_rows) => member_rows.iter().map(|r| r.get(0)).collect(),
                Err(e) => {
                    if verbose >= 1 {
                        println!("Warning: Failed to query role members for '{}': {}", target_role, e);
                    }
                    Vec::new()
                }
            };

            all_mappings.push(MappingRow {
                database: database.clone(),
                schema_name: row.get(0),
                target_role,
                granted_to,
                created_at: row.get(2),
                updated_at: row.get(3),
            });
        }
    }

    if all_mappings.is_empty() {
        println!("No schema-to-role mappings found in any database.");
        println!("Run 'init' command to set up the pattern in a database.");
        return Ok(());
    }

    // Display results
    println!("{:<20} {:<20} {:<30} {:<30} {:<21} {:<19}", "Database", "Schema", "Target Role", "Granted To", "Created At", "Updated At");
    println!("{}", "-".repeat(144));

    for mapping in &all_mappings {
        let truncated_role = truncate_with_ellipsis(&mapping.target_role, 30);
        let granted_display = if mapping.granted_to.is_empty() {
            "(none)".to_string()
        } else {
            mapping.granted_to.join(", ")
        };
        let truncated_granted = truncate_with_ellipsis(&granted_display, 30);

        println!(
            "{:<20} {:<20} {:<30} {:<30} {:<21} {:<19}",
            mapping.database,
            mapping.schema_name,
            truncated_role,
            truncated_granted,
            mapping.created_at.format("%Y-%m-%d %H:%M:%S"),
            mapping.updated_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    println!();
    println!("Total mappings: {} across {} database(s)", all_mappings.len(), databases_with_mappings.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── truncate_with_ellipsis ───────────────────────────────────────────────

    #[test]
    fn short_string_returned_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn string_exactly_max_len_returned_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn string_one_over_max_is_truncated() {
        let result = truncate_with_ellipsis("hello!", 5);
        assert!(result.ends_with("[...]"), "expected '[...]' suffix, got: {}", result);
    }

    #[test]
    fn truncated_result_is_no_longer_than_max_when_max_is_large() {
        let input = "a".repeat(100);
        let result = truncate_with_ellipsis(&input, 20);
        // saturating_sub(5) = 15 chars of prefix + 5 chars "[...]" = 20
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn max_len_of_five_produces_only_ellipsis_marker() {
        // saturating_sub(5) = 0, so prefix is empty → "[...]"
        let result = truncate_with_ellipsis("toolong", 5);
        assert_eq!(result, "[...]");
    }

    #[test]
    fn max_len_below_five_does_not_panic() {
        // saturating_sub(5) = 0 for any value ≤ 5; must not panic
        let _ = truncate_with_ellipsis("toolong", 0);
        let _ = truncate_with_ellipsis("toolong", 3);
    }
}
