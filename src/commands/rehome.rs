use anyhow::{Context, Result};
use tokio_postgres::Client;

use crate::db::{connect, ConnectionConfig};
use crate::report::{ActionOutcome, ActionReport};
use crate::utils::quote_identifier;

pub async fn execute(
    conn_opts: ConnectionConfig,
    database: String,
    source_schema: String,
    target_schema: String,
    verbose: u8,
) -> Result<()> {
    if source_schema == target_schema {
        anyhow::bail!(
            "Source and target schema are both '{}' — nothing to rehome.",
            source_schema,
        );
    }

    let mut report = ActionReport::new("Rehome");

    let mut config = conn_opts;
    config.dbname = Some(database.clone());
    let client = connect(&config).await?;

    // Source schema must exist — it's where the objects currently live.
    if !schema_exists(&client, &source_schema, verbose).await? {
        anyhow::bail!(
            "Source schema '{}' does not exist in database '{}'.",
            source_schema,
            database,
        );
    }

    // Target schema must already exist (should be set up via `init` first)
    if !schema_exists(&client, &target_schema, verbose).await? {
        anyhow::bail!(
            "Target schema '{}' does not exist. Create it first with \
             'pg-app-role-manager init --database {} --schema {} --role <role>'.",
            target_schema,
            database,
            target_schema,
        );
    }

    // Move in dependency-safe order
    move_types(&client, &source_schema, &target_schema, verbose, &mut report).await?;
    move_tables(&client, &source_schema, &target_schema, verbose, &mut report).await?;
    move_sequences(&client, &source_schema, &target_schema, verbose, &mut report).await?;
    move_views(&client, &source_schema, &target_schema, verbose, &mut report).await?;
    move_matviews(&client, &source_schema, &target_schema, verbose, &mut report).await?;
    move_functions(&client, &source_schema, &target_schema, verbose, &mut report).await?;

    report.print_summary();
    Ok(())
}

// ── Types (enums, ranges, non-table-backed composites, domains) ──────────────

async fn move_types(
    client: &Client,
    source_schema: &str,
    target_schema: &str,
    verbose: u8,
    report: &mut ActionReport,
) -> Result<()> {
    // Enums, ranges, non-table-backed composites, then domains.
    // Domains last because they may have CHECK constraints that reference
    // other types (which will have been moved already by this point).
    let sql = "\
        SELECT t.typname, t.typtype \
        FROM pg_type t \
        JOIN pg_namespace n ON t.typnamespace = n.oid \
        WHERE n.nspname = $1 \
          AND t.typtype IN ('e', 'r', 'c', 'd') \
          AND t.typrelid = 0 \
        ORDER BY \
          CASE t.typtype \
            WHEN 'e' THEN 1 \
            WHEN 'r' THEN 2 \
            WHEN 'c' THEN 3 \
            WHEN 'd' THEN 4 \
          END, \
          t.typname";

    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, source_schema);
    }

    let rows = client.query(sql, &[&source_schema]).await
        .with_context(|| format!("Failed to query types in '{}' schema", source_schema))?;

    for row in rows {
        let typname: String = row.get(0);
        let typtype: i8 = row.get(1);
        let kind = match typtype as u8 as char {
            'e' => "enum",
            'r' => "range",
            'c' => "composite type",
            'd' => "domain",
            _ => "type",
        };

        let alter = format!(
            "ALTER TYPE {}.{} SET SCHEMA {}",
            quote_identifier(source_schema),
            quote_identifier(&typname),
            quote_identifier(target_schema),
        );
        if verbose >= 1 {
            println!("[SQL] {}", alter);
        }
        client.execute(&alter, &[]).await
            .with_context(|| format!("Failed to move {} '{}' from '{}' to '{}'", kind, typname, source_schema, target_schema))?;
        report.record(format!("{} '{}.{}'", kind, source_schema, typname), ActionOutcome::Moved);
    }

    Ok(())
}

// ── Tables ───────────────────────────────────────────────────────────────────

async fn move_tables(
    client: &Client,
    source_schema: &str,
    target_schema: &str,
    verbose: u8,
    report: &mut ActionReport,
) -> Result<()> {
    let sql = "\
        SELECT tablename \
        FROM pg_tables \
        WHERE schemaname = $1 \
          AND tablename != 'schema_ownership_config' \
        ORDER BY tablename";

    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, source_schema);
    }

    let rows = client.query(sql, &[&source_schema]).await
        .with_context(|| format!("Failed to query tables in '{}' schema", source_schema))?;

    for row in rows {
        let tablename: String = row.get(0);
        let alter = format!(
            "ALTER TABLE {}.{} SET SCHEMA {}",
            quote_identifier(source_schema),
            quote_identifier(&tablename),
            quote_identifier(target_schema),
        );
        if verbose >= 1 {
            println!("[SQL] {}", alter);
        }
        client.execute(&alter, &[]).await
            .with_context(|| format!("Failed to move table '{}' from '{}' to '{}'", tablename, source_schema, target_schema))?;
        report.record(format!("table '{}.{}'", source_schema, tablename), ActionOutcome::Moved);
    }

    Ok(())
}

// ── Standalone sequences ─────────────────────────────────────────────────────

async fn move_sequences(
    client: &Client,
    source_schema: &str,
    target_schema: &str,
    verbose: u8,
    report: &mut ActionReport,
) -> Result<()> {
    // Skip column-owned sequences (deptype 'a' = SERIAL/BIGSERIAL, 'i' = IDENTITY).
    // Those are moved automatically when their table is moved.
    let sql = "\
        SELECT c.relname \
        FROM pg_class c \
        JOIN pg_namespace n ON c.relnamespace = n.oid \
        WHERE n.nspname = $1 \
          AND c.relkind = 'S' \
          AND NOT EXISTS ( \
            SELECT 1 FROM pg_depend d \
            WHERE d.objid = c.oid AND d.deptype IN ('a', 'i') \
          ) \
        ORDER BY c.relname";

    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, source_schema);
    }

    let rows = client.query(sql, &[&source_schema]).await
        .with_context(|| format!("Failed to query sequences in '{}' schema", source_schema))?;

    for row in rows {
        let seqname: String = row.get(0);
        let alter = format!(
            "ALTER SEQUENCE {}.{} SET SCHEMA {}",
            quote_identifier(source_schema),
            quote_identifier(&seqname),
            quote_identifier(target_schema),
        );
        if verbose >= 1 {
            println!("[SQL] {}", alter);
        }
        client.execute(&alter, &[]).await
            .with_context(|| format!("Failed to move sequence '{}' from '{}' to '{}'", seqname, source_schema, target_schema))?;
        report.record(format!("sequence '{}.{}'", source_schema, seqname), ActionOutcome::Moved);
    }

    Ok(())
}

// ── Views ────────────────────────────────────────────────────────────────────

async fn move_views(
    client: &Client,
    source_schema: &str,
    target_schema: &str,
    verbose: u8,
    report: &mut ActionReport,
) -> Result<()> {
    let sql = "SELECT viewname FROM pg_views WHERE schemaname = $1 ORDER BY viewname";

    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, source_schema);
    }

    let rows = client.query(sql, &[&source_schema]).await
        .with_context(|| format!("Failed to query views in '{}' schema", source_schema))?;

    for row in rows {
        let viewname: String = row.get(0);
        let alter = format!(
            "ALTER VIEW {}.{} SET SCHEMA {}",
            quote_identifier(source_schema),
            quote_identifier(&viewname),
            quote_identifier(target_schema),
        );
        if verbose >= 1 {
            println!("[SQL] {}", alter);
        }
        client.execute(&alter, &[]).await
            .with_context(|| format!("Failed to move view '{}' from '{}' to '{}'", viewname, source_schema, target_schema))?;
        report.record(format!("view '{}.{}'", source_schema, viewname), ActionOutcome::Moved);
    }

    Ok(())
}

// ── Materialized views ───────────────────────────────────────────────────────

async fn move_matviews(
    client: &Client,
    source_schema: &str,
    target_schema: &str,
    verbose: u8,
    report: &mut ActionReport,
) -> Result<()> {
    let sql = "SELECT matviewname FROM pg_matviews WHERE schemaname = $1 ORDER BY matviewname";

    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, source_schema);
    }

    let rows = client.query(sql, &[&source_schema]).await
        .with_context(|| format!("Failed to query materialized views in '{}' schema", source_schema))?;

    for row in rows {
        let matviewname: String = row.get(0);
        let alter = format!(
            "ALTER MATERIALIZED VIEW {}.{} SET SCHEMA {}",
            quote_identifier(source_schema),
            quote_identifier(&matviewname),
            quote_identifier(target_schema),
        );
        if verbose >= 1 {
            println!("[SQL] {}", alter);
        }
        client.execute(&alter, &[]).await
            .with_context(|| format!("Failed to move materialized view '{}' from '{}' to '{}'", matviewname, source_schema, target_schema))?;
        report.record(format!("materialized view '{}.{}'", source_schema, matviewname), ActionOutcome::Moved);
    }

    Ok(())
}

// ── Functions and procedures ─────────────────────────────────────────────────

async fn move_functions(
    client: &Client,
    source_schema: &str,
    target_schema: &str,
    verbose: u8,
    report: &mut ActionReport,
) -> Result<()> {
    let sql = "\
        SELECT p.proname, pg_get_function_identity_arguments(p.oid) AS args, p.prokind \
        FROM pg_proc p \
        JOIN pg_namespace n ON p.pronamespace = n.oid \
        WHERE n.nspname = $1 \
          AND p.proname != 'auto_transfer_schema_ownership' \
        ORDER BY p.proname, args";

    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, source_schema);
    }

    let rows = client.query(sql, &[&source_schema]).await
        .with_context(|| format!("Failed to query functions in '{}' schema", source_schema))?;

    for row in rows {
        let proname: String = row.get(0);
        let args: String = row.get(1);
        let prokind: i8 = row.get(2);

        let (alter_kind, display_kind) = if prokind as u8 as char == 'p' {
            ("PROCEDURE", "procedure")
        } else {
            ("FUNCTION", "function")
        };

        let alter = format!(
            "ALTER {} {}.{}({}) SET SCHEMA {}",
            alter_kind,
            quote_identifier(source_schema),
            quote_identifier(&proname),
            args,
            quote_identifier(target_schema),
        );
        if verbose >= 1 {
            println!("[SQL] {}", alter);
        }
        client.execute(&alter, &[]).await
            .with_context(|| format!("Failed to move {} '{}' from '{}' to '{}'", display_kind, proname, source_schema, target_schema))?;
        report.record(format!("{} '{}.{}({})'", display_kind, source_schema, proname, args), ActionOutcome::Moved);
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn schema_exists(client: &Client, schema: &str, verbose: u8) -> Result<bool> {
    let sql = "SELECT 1 FROM pg_namespace WHERE nspname = $1";
    if verbose >= 1 {
        println!("[SQL] {} -- params: [{}]", sql, schema);
    }
    Ok(client.query_one(sql, &[&schema]).await.is_ok())
}
