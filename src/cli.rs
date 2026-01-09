use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pg-app-role-manager")]
#[command(about = "PostgreSQL schema ownership pattern manager", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub connection: ConnectionOpts,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub struct ConnectionOpts {
    #[arg(long, env = "PGHOST", default_value = "localhost")]
    pub host: String,

    #[arg(long, env = "PGPORT", default_value = "5432")]
    pub port: u16,

    #[arg(long, env = "PGUSER", required = true)]
    pub user: String,

    #[arg(long, env = "PGPASSWORD", required = true, hide_env_values = true)]
    pub password: String,

    #[arg(long, env = "PGDATABASE")]
    pub dbname: Option<String>,

    #[arg(long, env = "PGSSLMODE", default_value = "prefer", help = "SSL mode: disable, prefer, or require")]
    pub sslmode: String,

    #[arg(short = 'v', action = ArgAction::Count, help = "Increase verbosity (-v for SQL statements, -vv includes trigger function)")]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Command {
    Init {
        #[arg(long)]
        database: Option<String>,

        #[arg(long, required = true)]
        schema: String,

        #[arg(long, required = true)]
        role: String,
    },
    ListMappings,
    Version,
}
