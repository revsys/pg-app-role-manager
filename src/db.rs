use anyhow::{Context, Result};
use postgres_rustls::MakeTlsConnector;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

#[derive(Clone, Debug)]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
}

impl SslMode {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "disable" => Ok(SslMode::Disable),
            "prefer" => Ok(SslMode::Prefer),
            "require" => Ok(SslMode::Require),
            _ => Err(anyhow::anyhow!(
                "Invalid SSL mode '{}'. Valid options are: disable, prefer, require.",
                s
            )),
        }
    }
}

impl Default for SslMode {
    fn default() -> Self {
        SslMode::Prefer
    }
}

#[derive(Clone)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: Option<String>,
    pub sslmode: SslMode,
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

/// Custom certificate verifier that accepts all certificates without validation.
/// This matches PostgreSQL's "require" sslmode: encryption required but no cert verification.
#[derive(Debug)]
struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Accept all certificates without verification
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Accept all signatures without verification
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Accept all signatures without verification
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // Support all signature schemes
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

fn create_tls_connector() -> Result<MakeTlsConnector> {
    // Build rustls ClientConfig without certificate verification
    // This matches PostgreSQL's "require" mode: encryption required, no cert verification
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth();

    // CRITICAL: Set PostgreSQL ALPN protocol
    postgres_rustls::set_postgresql_alpn(&mut config);

    // Create tokio-rustls connector and wrap for postgres
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    Ok(MakeTlsConnector::new(tls_connector))
}

pub async fn connect(config: &ConnectionConfig) -> Result<Client> {
    let conn_str = config.build_connection_string();

    match config.sslmode {
        SslMode::Disable => {
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
        SslMode::Require => {
            let tls_connector = create_tls_connector()?;

            let (client, connection) = tokio_postgres::connect(&conn_str, tls_connector)
                .await
                .context("Failed to connect to PostgreSQL with required TLS")?;

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            Ok(client)
        }
        SslMode::Prefer => {
            // Try TLS connection first
            let tls_connector = create_tls_connector()?;

            match tokio_postgres::connect(&conn_str, tls_connector).await {
                Ok((client, connection)) => {
                    // TLS connection succeeded
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            eprintln!("Connection error: {}", e);
                        }
                    });
                    Ok(client)
                }
                Err(tls_err) => {
                    // TLS connection failed, log warning and try without TLS
                    eprintln!("Warning: TLS connection failed ({}), falling back to unencrypted connection", tls_err);

                    let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
                        .await
                        .context("Failed to connect to PostgreSQL without TLS")?;

                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            eprintln!("Connection error: {}", e);
                        }
                    });

                    Ok(client)
                }
            }
        }
    }
}
