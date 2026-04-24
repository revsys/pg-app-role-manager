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
    Rehome {
        #[arg(long)]
        database: Option<String>,

        #[arg(long, required = true)]
        schema: String,
    },
    Version,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize tests that manipulate env vars (set_var/remove_var are unsafe
    // in Rust 2024 due to thread-safety; the mutex prevents races).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // ── Successful parses ────────────────────────────────────────────────────

    #[test]
    fn init_parses_all_fields() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p",
            "init", "--schema", "myschema", "--role", "myrole",
        ]).unwrap();
        assert_eq!(cli.connection.user, "u");
        assert_eq!(cli.connection.password, "p");
        match cli.command {
            Command::Init { schema, role, .. } => {
                assert_eq!(schema, "myschema");
                assert_eq!(role, "myrole");
            }
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn init_with_optional_database() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p",
            "init", "--database", "mydb", "--schema", "s", "--role", "r",
        ]).unwrap();
        match cli.command {
            Command::Init { database, .. } => assert_eq!(database.as_deref(), Some("mydb")),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn rehome_parses_schema_without_database() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p",
            "rehome", "--schema", "app",
        ]).unwrap();
        match cli.command {
            Command::Rehome { database, schema } => {
                assert_eq!(schema, "app");
                assert!(database.is_none());
            }
            _ => panic!("expected Rehome"),
        }
    }

    #[test]
    fn rehome_with_database() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p",
            "rehome", "--database", "mydb", "--schema", "app",
        ]).unwrap();
        match cli.command {
            Command::Rehome { database, schema } => {
                assert_eq!(schema, "app");
                assert_eq!(database.as_deref(), Some("mydb"));
            }
            _ => panic!("expected Rehome"),
        }
    }

    #[test]
    fn list_mappings_parses() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "list-mappings",
        ]).unwrap();
        assert!(matches!(cli.command, Command::ListMappings));
    }

    #[test]
    fn version_parses() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "version",
        ]).unwrap();
        assert!(matches!(cli.command, Command::Version));
    }

    // ── Verbosity counting ───────────────────────────────────────────────────

    #[test]
    fn verbosity_absent_is_zero() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "version",
        ]).unwrap();
        assert_eq!(cli.connection.verbose, 0);
    }

    #[test]
    fn single_v_is_one() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "-v", "version",
        ]).unwrap();
        assert_eq!(cli.connection.verbose, 1);
    }

    #[test]
    fn double_v_is_two() {
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "-vv", "version",
        ]).unwrap();
        assert_eq!(cli.connection.verbose, 2);
    }

    // ── Required args: --schema and --role have no env fallback, so these
    //   tests are reliable regardless of the shell environment.  ─────────────

    #[test]
    fn init_requires_schema() {
        assert!(Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p",
            "init", "--role", "r",
        ]).is_err());
    }

    #[test]
    fn init_requires_role() {
        assert!(Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p",
            "init", "--schema", "s",
        ]).is_err());
    }

    #[test]
    fn rehome_requires_schema() {
        assert!(Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "rehome",
        ]).is_err());
    }

    // ── Required args that have env fallbacks: serialize via ENV_LOCK ────────

    #[test]
    fn missing_user_rejected_without_env_var() {
        let _guard = ENV_LOCK.lock().unwrap();
        let saved = std::env::var("PGUSER").ok();
        // SAFETY: single-threaded section guarded by ENV_LOCK
        unsafe { std::env::remove_var("PGUSER"); }
        let result = Cli::try_parse_from(["prog", "--password", "p", "version"]);
        if let Some(v) = saved { unsafe { std::env::set_var("PGUSER", v); } }
        assert!(result.is_err());
    }

    #[test]
    fn missing_password_rejected_without_env_var() {
        let _guard = ENV_LOCK.lock().unwrap();
        let saved = std::env::var("PGPASSWORD").ok();
        unsafe { std::env::remove_var("PGPASSWORD"); }
        let result = Cli::try_parse_from(["prog", "--user", "u", "version"]);
        if let Some(v) = saved { unsafe { std::env::set_var("PGPASSWORD", v); } }
        assert!(result.is_err());
    }

    // ── Default connection option values ────────────────────────────────────

    #[test]
    fn default_host_and_port() {
        let _guard = ENV_LOCK.lock().unwrap();
        let saved_host = std::env::var("PGHOST").ok();
        let saved_port = std::env::var("PGPORT").ok();
        unsafe {
            std::env::remove_var("PGHOST");
            std::env::remove_var("PGPORT");
        }
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "version",
        ]).unwrap();
        if let Some(v) = saved_host { unsafe { std::env::set_var("PGHOST", v); } }
        if let Some(v) = saved_port { unsafe { std::env::set_var("PGPORT", v); } }
        assert_eq!(cli.connection.host, "localhost");
        assert_eq!(cli.connection.port, 5432);
    }

    #[test]
    fn default_sslmode() {
        let _guard = ENV_LOCK.lock().unwrap();
        let saved = std::env::var("PGSSLMODE").ok();
        unsafe { std::env::remove_var("PGSSLMODE"); }
        let cli = Cli::try_parse_from([
            "prog", "--user", "u", "--password", "p", "version",
        ]).unwrap();
        if let Some(v) = saved { unsafe { std::env::set_var("PGSSLMODE", v); } }
        assert_eq!(cli.connection.sslmode, "prefer");
    }
}
