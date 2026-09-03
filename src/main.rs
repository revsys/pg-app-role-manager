mod cli;
mod commands;
mod db;
mod report;
mod sql_templates;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use db::{ConnectionConfig, SslMode};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    if let Command::Version = args.command {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let user = args.connection.user
        .ok_or_else(|| anyhow::anyhow!("--user / PGUSER is required"))?;
    let password = args.connection.password
        .ok_or_else(|| anyhow::anyhow!("--password / PGPASSWORD is required"))?;

    // Parse SSL mode
    let sslmode = SslMode::from_str(&args.connection.sslmode)?;

    let conn_config = ConnectionConfig {
        host: args.connection.host,
        port: args.connection.port,
        user,
        password,
        dbname: args.connection.dbname,
        sslmode,
    };

    let verbose = args.connection.verbose;

    match args.command {
        Command::Init { database, schema, role, read_only_role } => {
            // Resolve database name from --database flag or PGDATABASE env var
            let resolved_database = database.or_else(|| conn_config.dbname.clone())
                .ok_or_else(|| anyhow::anyhow!(
                    "Database must be specified via --database flag or PGDATABASE environment variable"
                ))?;
            let resolved_ro_role = read_only_role.unwrap_or_else(|| format!("{}_ro", schema));

            commands::init::execute(conn_config, resolved_database, schema, role, resolved_ro_role, verbose).await?;
        }
        Command::ListMappings => {
            commands::list_mappings::execute(conn_config, verbose).await?;
        }
        Command::Rehome { database, source_schema, target_schema } => {
            let resolved_database = database.or_else(|| conn_config.dbname.clone())
                .ok_or_else(|| anyhow::anyhow!(
                    "Database must be specified via --database flag or PGDATABASE environment variable"
                ))?;

            commands::rehome::execute(conn_config, resolved_database, source_schema, target_schema, verbose).await?;
        }
        Command::Version => unreachable!(),
    }

    Ok(())
}
