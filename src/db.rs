use anyhow::{Context, Result};
use tokio_postgres::{Client, NoTls};

#[derive(Clone)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: Option<String>,
}

impl ConnectionConfig {
    pub fn build_connection_string(&self) -> String {
        let dbname = self.dbname.as_deref().unwrap_or("postgres");
        format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, dbname
        )
    }
}

pub async fn connect(config: &ConnectionConfig) -> Result<Client> {
    let conn_str = config.build_connection_string();

    let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
        .await
        .context("Failed to connect to PostgreSQL")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    Ok(client)
}
