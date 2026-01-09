mod cli;
mod commands;
mod db;
mod report;
mod sql_templates;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use db::{ConnectionConfig, SslMode};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Parse SSL mode
    let sslmode = SslMode::from_str(&args.connection.sslmode)?;

    let conn_config = ConnectionConfig {
        host: args.connection.host,
        port: args.connection.port,
        user: args.connection.user,
        password: args.connection.password,
        dbname: args.connection.dbname,
        sslmode,
    };

    let verbose = args.connection.verbose;

    match args.command {
        Command::Init { database, schema, role } => {
            // Resolve database name from --database flag or PGDATABASE env var
            let resolved_database = database.or_else(|| conn_config.dbname.clone())
                .ok_or_else(|| anyhow::anyhow!(
                    "Database must be specified via --database flag or PGDATABASE environment variable"
                ))?;

            commands::init::execute(conn_config, resolved_database, schema, role, verbose).await?;
        }
        Command::ListMappings => {
            commands::list_mappings::execute(conn_config, verbose).await?;
        }
        Command::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
