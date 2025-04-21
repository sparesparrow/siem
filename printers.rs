use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::{Result, anyhow};
use tracing::{info, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Printer {
    pub id: Uuid,
    pub name: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub model: String,
    pub location: String,
    pub status: PrinterStatus,
    pub last_seen: DateTime<Utc>,
    pub supplies: Vec<PrinterSupply>,
    pub capabilities: PrinterCapabilities,
    pub queue_status: Vec<PrintJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrinterStatus {
    Online,
    Offline,
    Error,
    Warning,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterSupply {
    pub supply_type: SupplyType,
    pub name: String,
    pub level: u8, // 0-100%
    pub status: SupplyStatus,
    pub capacity: Option<u32>, // Pages or ml
    pub last_replaced: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SupplyType {
    Toner,
    Ink,
    Drum,
    Fuser,
    TransferBelt,
    WasteToner,
    Staples,
    Paper,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SupplyStatus {
    OK,
    Low,
    Empty,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterCapabilities {
    pub color: bool,
    pub duplex: bool,
    pub paper_sizes: Vec<String>,
    pub scanner: bool,
    pub fax: bool,
    pub pages_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: String,
    pub name: String,
    pub user: String,
    pub submitted_at: DateTime<Utc>,
    pub pages: Option<u32>,
    pub status: PrintJobStatus,
    pub size_kb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrintJobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

pub struct PrinterManager {
    printers: HashMap<Uuid, Printer>,
}

impl PrinterManager {
    pub fn new() -> Self {
        PrinterManager {
            printers: HashMap::new(),
        }
    }
    
    pub fn add_printer(&mut self, printer: Printer) -> Result<Uuid> {
        // Check if printer with the same IP already exists
        if self.printers.values().any(|p| p.ip_address == printer.ip_address) {
            return Err(anyhow!("A printer with IP {} already exists", printer.ip_address));
        }
        
        let id = printer.id;
        self.printers.insert(id, printer);
        
        info!("Added printer with ID: {}", id);
        Ok(id)
    }
    
    pub fn get_printer(&self, id: &Uuid) -> Option<&Printer> {
        self.printers.get(id)
    }
    
    pub fn get_printers(&self) -> Vec<&Printer> {
        self.printers.values().collect()
    }
    
    pub fn update_printer(&mut self, id: &Uuid, updated_printer: Printer) -> Result<()> {
        if !self.printers.contains_key(id) {
            return Err(anyhow!("Printer not found: {}", id));
        }
        
        // Check if we're trying to update to an IP that's already in use by another printer
        if self.printers.values().any(|p| p.id != *id && p.ip_address == updated_printer.ip_address) {
            return Err(anyhow!("A printer with IP {} already exists", updated_printer.ip_address));
        }
        
        self.printers.insert(*id, updated_printer);
        
        info!("Updated printer: {}", id);
        Ok(())
    }
    
    pub fn delete_printer(&mut self, id: &Uuid) -> Result<()> {
        if self.printers.remove(id).is_none() {
            return Err(anyhow!("Printer not found: {}", id));
        }
        
        info!("Deleted printer: {}", id);
        Ok(())
    }
    
    pub fn update_printer_status(&mut self, id: &Uuid, status: PrinterStatus) -> Result<()> {
        let printer = self.printers.get_mut(id)
            .ok_or_else(|| anyhow!("Printer not found: {}", id))?;
        
        let status_clone = status.clone();
        printer.status = status;
        printer.last_seen = Utc::now();
        
        info!("Updated printer status for {}: {:?}", id, status_clone);
        Ok(())
    }
    
    pub fn update_supply_levels(&mut self, id: &Uuid, supplies: Vec<PrinterSupply>) -> Result<()> {
        let printer = self.printers.get_mut(id)
            .ok_or_else(|| anyhow!("Printer not found: {}", id))?;
        
        printer.supplies = supplies;
        printer.last_seen = Utc::now();
        
        info!("Updated supplies for printer: {}", id);
        Ok(())
    }
    
    pub fn add_print_job(&mut self, id: &Uuid, job: PrintJob) -> Result<()> {
        let printer = self.printers.get_mut(id)
            .ok_or_else(|| anyhow!("Printer not found: {}", id))?;
        
        printer.queue_status.push(job);
        
        info!("Added print job to printer: {}", id);
        Ok(())
    }
    
    pub fn update_print_job(&mut self, id: &Uuid, job_id: &str, status: PrintJobStatus) -> Result<()> {
        let printer = self.printers.get_mut(id)
            .ok_or_else(|| anyhow!("Printer not found: {}", id))?;
        
        if let Some(job) = printer.queue_status.iter_mut().find(|j| j.id == job_id) {
            let status_clone = status.clone();
            job.status = status;
            
            info!("Updated print job {} status to {:?}", job_id, status_clone);
            Ok(())
        } else {
            Err(anyhow!("Print job not found: {}", job_id))
        }
    }
    
    pub fn clean_completed_jobs(&mut self, id: &Uuid, older_than_hours: u32) -> Result<u32> {
        let printer = self.printers.get_mut(id)
            .ok_or_else(|| anyhow!("Printer not found: {}", id))?;
        
        let cutoff = Utc::now() - chrono::Duration::hours(older_than_hours as i64);
        let old_len = printer.queue_status.len();
        
        printer.queue_status.retain(|job| 
            !(job.status == PrintJobStatus::Completed && job.submitted_at < cutoff)
        );
        
        let removed = old_len - printer.queue_status.len();
        
        info!("Cleaned up {} completed jobs from printer: {}", removed, id);
        Ok(removed as u32)
    }
}

pub fn start() -> Result<PrinterManager> {
    let manager = PrinterManager::new();
    info!("Printer manager started");
    Ok(manager)
}


// New module for logging and reporting

mod logging {
    use crate::models::{LogEntry, LogSeverity};
    use chrono::Utc;
    use std::fmt::Write;
    use tracing::{info, warn, error};

    pub fn format_log_entry(entry: &LogEntry) -> String {
        let mut output = String::new();

        writeln!(&mut output, "======== LOG ENTRY ========").unwrap();
        writeln!(&mut output, "ID: {}", entry.id).unwrap();
        writeln!(&mut output, "Timestamp: {}", entry.timestamp).unwrap();
        writeln!(&mut output, "Source: {}", entry.source).unwrap();
        writeln!(&mut output, "Type: {}", entry.event_type).unwrap();
        writeln!(&mut output, "Severity: {:?}", entry.severity).unwrap();
        writeln!(&mut output, "Message: {}", entry.message).unwrap();

        if let Some(host) = &entry.host {
            writeln!(&mut output, "Host: {}", host).unwrap();
        }

        if let Some(user) = &entry.user {
            writeln!(&mut output, "User: {}", user).unwrap();
        }

        if let Some(app) = &entry.application {
            writeln!(&mut output, "Application: {}", app).unwrap();
        }

        if !entry.tags.is_empty() {
            writeln!(&mut output, "Tags: {}", entry.tags.join(", ")).unwrap();
        }

        writeln!(&mut output, "Raw Data: {}", entry.raw_data).unwrap();
        writeln!(&mut output, "==========================").unwrap();

        output
    }

    pub fn print_log_entry(entry: &LogEntry) {
        match entry.severity {
            LogSeverity::Debug | LogSeverity::Info => {
                info!("{}", format_log_entry(entry));
            },
            LogSeverity::Warning => {
                warn!("{}", format_log_entry(entry));
            },
            LogSeverity::Error | LogSeverity::Critical => {
                error!("{}", format_log_entry(entry));
            },
        }
    }

    pub fn generate_incident_report(entries: &[LogEntry], title: &str) -> String {
        let mut report = String::new();
        let now = Utc::now();

        writeln!(&mut report, "========================================").unwrap();
        writeln!(&mut report, "INCIDENT REPORT: {}", title).unwrap();
        writeln!(&mut report, "Generated at: {}", now).unwrap();
        writeln!(&mut report, "Total Events: {}", entries.len()).unwrap();
        writeln!(&mut report, "========================================").unwrap();
        writeln!(&mut report).unwrap();

        // Summary by severity
        let mut debug_count = 0;
        let mut info_count = 0;
        let mut warning_count = 0;
        let mut error_count = 0;
        let mut critical_count = 0;

        for entry in entries {
            match entry.severity {
                LogSeverity::Debug => debug_count += 1,
                LogSeverity::Info => info_count += 1,
                LogSeverity::Warning => warning_count += 1,
                LogSeverity::Error => error_count += 1,
                LogSeverity::Critical => critical_count += 1,
            }
        }

        writeln!(&mut report, "SEVERITY SUMMARY:").unwrap();
        writeln!(&mut report, "Debug:    {}", debug_count).unwrap();
        writeln!(&mut report, "Info:     {}", info_count).unwrap();
        writeln!(&mut report, "Warning:  {}", warning_count).unwrap();
        writeln!(&mut report, "Error:    {}", error_count).unwrap();
        writeln!(&mut report, "Critical: {}", critical_count).unwrap();
        writeln!(&mut report).unwrap();

        // List critical and error events first
        if critical_count > 0 || error_count > 0 {
            writeln!(&mut report, "CRITICAL AND ERROR EVENTS:").unwrap();
            for entry in entries {
                if entry.severity == LogSeverity::Critical || entry.severity == LogSeverity::Error {
                    writeln!(&mut report, "- [{}] {} ({}): {}", 
                             entry.timestamp, entry.source, entry.severity, entry.message).unwrap();
                }
            }
            writeln!(&mut report).unwrap();
        }

        // Timeline of all events
        writeln!(&mut report, "EVENT TIMELINE:").unwrap();
        for entry in entries {
            writeln!(&mut report, "[{}] {} - {}: {}", 
                     entry.timestamp, entry.severity, entry.source, entry.message).unwrap();
        }

        report
    }

    pub fn generate_compliance_report(entries: &[LogEntry], start_date: chrono::DateTime<Utc>, end_date: chrono::DateTime<Utc>) -> String {
        let mut report = String::new();
        let now = Utc::now();

        writeln!(&mut report, "========================================").unwrap();
        writeln!(&mut report, "COMPLIANCE REPORT").unwrap();
        writeln!(&mut report, "Generated at: {}", now).unwrap();
        writeln!(&mut report, "Period: {} to {}", start_date, end_date).unwrap();
        writeln!(&mut report, "========================================").unwrap();
        writeln!(&mut report).unwrap();

        // Filter events in the date range
        let filtered_entries: Vec<_> = entries.iter()
            .filter(|e| e.timestamp >= start_date && e.timestamp <= end_date)
            .collect();

        writeln!(&mut report, "Total Events in Period: {}", filtered_entries.len()).unwrap();
        writeln!(&mut report).unwrap();

        // Security incidents summary
        let security_incidents: Vec<_> = filtered_entries.iter()
            .filter(|e| e.event_type.contains("security") && 
                   (e.severity == LogSeverity::Error || e.severity == LogSeverity::Critical))
            .collect();

        writeln!(&mut report, "SECURITY INCIDENTS: {}", security_incidents.len()).unwrap();
        for incident in &security_incidents {
            writeln!(&mut report, "- [{}] {}: {}", 
                     incident.timestamp, incident.source, incident.message).unwrap();
        }
        writeln!(&mut report).unwrap();

        // Access control events
        let access_events: Vec<_> = filtered_entries.iter()
            .filter(|e| e.event_type.contains("access") || e.event_type.contains("authentication"))
            .collect();

        writeln!(&mut report, "ACCESS CONTROL EVENTS: {}", access_events.len()).unwrap();
        let failed_access = access_events.iter()
            .filter(|e| e.message.contains("failed") || e.message.contains("denied"))
            .count();

        writeln!(&mut report, "Failed Access Attempts: {}", failed_access).unwrap();
        writeln!(&mut report).unwrap();

        // System availability
        let availability_incidents: Vec<_> = filtered_entries.iter()
            .filter(|e| e.event_type.contains("availability") && e.severity == LogSeverity::Critical)
            .collect();

        writeln!(&mut report, "AVAILABILITY INCIDENTS: {}", availability_incidents.len()).unwrap();
        for incident in &availability_incidents {
            writeln!(&mut report, "- [{}] {}: {}", 
                     incident.timestamp, incident.source, incident.message).unwrap();
        }
        writeln!(&mut report).unwrap();

        // Compliance summary
        writeln!(&mut report, "COMPLIANCE SUMMARY:").unwrap();
        writeln!(&mut report, "- Security Incident Rate: {:.2}%", 
                if filtered_entries.is_empty() { 0.0 } else { (security_incidents.len() as f64 / filtered_entries.len() as f64) * 100.0 }).unwrap();
        writeln!(&mut report, "- Failed Access Rate: {:.2}%", 
                if access_events.is_empty() { 0.0 } else { (failed_access as f64 / access_events.len() as f64) * 100.0 }).unwrap();

        report
    }
}