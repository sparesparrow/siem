use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Technician,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub event_type: String,
    pub severity: LogSeverity,
    pub message: String,
    pub raw_data: String,
    pub host: Option<String>,
    pub user: Option<String>,
    pub application: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for LogSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSeverity::Debug => write!(f, "DEBUG"),
            LogSeverity::Info => write!(f, "INFO"),
            LogSeverity::Warning => write!(f, "WARNING"),
            LogSeverity::Error => write!(f, "ERROR"),
            LogSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub status: AlertStatus,
    pub source: String,
    pub related_logs: Vec<Uuid>,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    New,
    Acknowledged,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub name: String,
    pub asset_type: AssetType,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub operating_system: Option<String>,
    pub owner: Option<String>,
    pub location: Option<String>,
    pub purchase_date: Option<DateTime<Utc>>,
    pub status: AssetStatus,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    Workstation,
    Server,
    Laptop,
    NetworkDevice,
    Printer,
    Mobile,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetStatus {
    Active,
    Maintenance,
    Retired,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkScan {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub target_ip: String,
    pub scan_type: String,
    pub status: ScanStatus,
    pub findings: Vec<ScanFinding>,
    pub initiated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanStatus {
    Scheduled,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFinding {
    pub id: Uuid,
    pub ip_address: String,
    pub port: Option<u16>,
    pub service: Option<String>,
    pub severity: FindingSeverity,
    pub description: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindingSeverity {
    Informational,
    Low,
    Medium,
    High,
    Critical,
}