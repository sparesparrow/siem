
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::net::IpAddr;
use std::str::FromStr;
use tracing::{info, error};
use uuid::Uuid;

use crate::models::LogEntry;

// Database configuration
#[derive(Clone)]
pub struct DatabaseManager {
    pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to database at {}", database_url);
        
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
            
        info!("Database connection established");
        
        // Initialize tables if they don't exist
        Self::initialize_tables(&pool).await?;
        
        Ok(Self { pool })
    }
    
    async fn initialize_tables(pool: &PgPool) -> Result<()> {
        info!("Initializing database tables...");
        
        // Create logs table with specialized IP address column
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS logs (
                id UUID PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                ip_address INET,
                log_message TEXT NOT NULL,
                log_level TEXT NOT NULL,
                source TEXT NOT NULL,
                raw_data TEXT NOT NULL,
                host TEXT,
                user_id TEXT,
                application TEXT,
                tags TEXT[]
            );
            
            -- Create indexes for efficient querying
            CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs (timestamp);
            CREATE INDEX IF NOT EXISTS idx_logs_ip_address ON logs (ip_address);
            CREATE INDEX IF NOT EXISTS idx_logs_log_level ON logs (log_level);
            CREATE INDEX IF NOT EXISTS idx_logs_source ON logs (source);
        "#)
        .execute(pool)
        .await?;
        
        info!("Database tables initialized successfully");
        Ok(())
    }
    
    pub async fn store_log(&self, entry: &LogEntry) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO logs (
                id, timestamp, ip_address, log_message, log_level, 
                source, raw_data, host, user_id, application, tags
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
        "#)
        .bind(entry.id)
        .bind(entry.timestamp)
        .bind(entry.host.as_ref().and_then(|h| IpAddr::from_str(h).ok().map(|ip| ip.to_string())).unwrap_or_default())
        .bind(&entry.message)
        .bind(entry.severity.to_string())
        .bind(&entry.source)
        .bind(&entry.raw_data)
        .bind(&entry.host)
        .bind(&entry.user)
        .bind(&entry.application)
        .bind(&entry.tags)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn query_logs_by_ip(&self, ip_address: &str) -> Result<Vec<LogEntry>> {
        let logs = sqlx::query_as!(
            LogEntryRow,
            r#"
            SELECT id, timestamp, ip_address, log_message, log_level, 
                   source, raw_data, host, user_id as user, application, tags
            FROM logs
            WHERE ip_address = $1::inet
            ORDER BY timestamp DESC
            "#,
            ip_address
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(logs.into_iter().map(|row| row.into()).collect())
    }
    
    pub async fn query_logs_by_ip_range(&self, ip_range: &str) -> Result<Vec<LogEntry>> {
        let logs = sqlx::query_as!(
            LogEntryRow,
            r#"
            SELECT id, timestamp, ip_address, log_message, log_level, 
                   source, raw_data, host, user_id as user, application, tags
            FROM logs
            WHERE ip_address <<= $1::inet
            ORDER BY timestamp DESC
            "#,
            ip_range
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(logs.into_iter().map(|row| row.into()).collect())
    }
}

// Database row representation matching the logs table
#[derive(Debug, Serialize, Deserialize)]
struct LogEntryRow {
    id: Uuid,
    timestamp: chrono::DateTime<Utc>,
    ip_address: Option<String>,
    log_message: String,
    log_level: String,
    source: String,
    raw_data: String,
    host: Option<String>,
    user: Option<String>,
    application: Option<String>,
    tags: Option<Vec<String>>,
}

// Convert from database row to LogEntry model
impl From<LogEntryRow> for LogEntry {
    fn from(row: LogEntryRow) -> Self {
        use crate::models::LogSeverity;
        
        let severity = match row.log_level.as_str() {
            "ERROR" => LogSeverity::Error,
            "WARNING" => LogSeverity::Warning,
            "INFO" => LogSeverity::Info,
            "DEBUG" => LogSeverity::Debug,
            _ => LogSeverity::Info,
        };
        
        LogEntry {
            id: row.id,
            timestamp: row.timestamp,
            source: row.source,
            event_type: "".to_string(), // This would need to be mapped or added to the schema
            severity,
            message: row.log_message,
            raw_data: row.raw_data,
            host: row.host,
            user: row.user,
            application: row.application,
            tags: row.tags.unwrap_or_default(),
        }
    }
}
