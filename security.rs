use aes::{Aes256, cipher::{BlockEncrypt, BlockDecrypt}};
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};

#[derive(Clone)]
pub struct SecurityManager {
    key: [u8; 32],
    audit_log: Arc<Mutex<Vec<AuditEvent>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<Utc>,
    pub user: String,
    pub action: String,
    pub resource: String,
    pub status: AuditStatus,
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum AuditStatus {
    Success,
    Failure,
    Warning,
}

impl SecurityManager {
    pub fn new(key: [u8; 32]) -> Self {
        Self { 
            key,
            audit_log: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub fn encrypt_data(&self, data: &str) -> String {
        // This is a simplified implementation for demonstration
        // In production, use a proper encryption method with IV, etc.
        general_purpose::STANDARD.encode(data.as_bytes())
    }

    pub fn decrypt_data(&self, encrypted_data: &str) -> Result<String, String> {
        // This is a simplified implementation for demonstration
        match general_purpose::STANDARD.decode(encrypted_data) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => Ok(s),
                Err(_) => Err("Invalid UTF-8 data".to_string()),
            },
            Err(_) => Err("Invalid base64 data".to_string()),
        }
    }

    pub fn log_audit_event(&self, user: &str, action: &str, resource: &str, status: AuditStatus, details: Option<String>) {
        let details_clone = details.clone(); // Clone it first to avoid the move
        
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            user: user.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            status,
            details,
        };

        match self.audit_log.lock() {
            Ok(mut log) => {
                log.push(event.clone());

                // Log to tracing based on status
                match event.status {
                    AuditStatus::Success => info!(
                        "AUDIT: [{}] User '{}' performed '{}' on '{}': Success",
                        event.id, user, action, resource
                    ),
                    AuditStatus::Warning => warn!(
                        "AUDIT: [{}] User '{}' performed '{}' on '{}': Warning: {}",
                        event.id, user, action, resource, details_clone.unwrap_or_default()
                    ),
                    AuditStatus::Failure => error!(
                        "AUDIT: [{}] User '{}' attempted '{}' on '{}': Failed: {}",
                        event.id, user, action, resource, details_clone.unwrap_or_default()
                    ),
                }
            },
            Err(e) => {
                error!("Failed to log audit event: {:?}", e);
            }
        }
    }

    pub fn get_audit_logs(&self) -> Vec<AuditEvent> {
        match self.audit_log.lock() {
            Ok(log) => log.clone(),
            Err(_) => {
                error!("Failed to access audit logs");
                Vec::new()
            }
        }
    }

    pub fn verify_access(&self, user: &str, resource: &str, action: &str) -> bool {
        // This is a simplified access control check
        // In production, use a proper RBAC system

        // For demonstration, all actions are allowed
        self.log_audit_event(
            user,
            action,
            resource,
            AuditStatus::Success,
            None,
        );

        true
    }
}

// Access control implementation
pub struct AccessControl {
    permissions: HashMap<String, Vec<String>>,
}

impl AccessControl {
    pub fn new() -> Self {
        let mut ac = Self {
            permissions: HashMap::new(),
        };
        
        // Set up default permissions
        ac.permissions.insert("admin".to_string(), vec![
            "script:read".to_string(),
            "script:write".to_string(),
            "script:execute".to_string(),
            "ticket:read".to_string(),
            "ticket:write".to_string(),
            "printer:read".to_string(),
            "printer:manage".to_string(),
            "user:read".to_string(),
            "user:write".to_string(),
        ]);
        
        ac.permissions.insert("technician".to_string(), vec![
            "script:read".to_string(),
            "script:execute".to_string(),
            "ticket:read".to_string(),
            "ticket:write".to_string(),
            "printer:read".to_string(),
        ]);
        
        ac.permissions.insert("user".to_string(), vec![
            "ticket:read_own".to_string(),
            "ticket:create".to_string(),
        ]);
        
        ac
    }
    
    pub fn check_permission(&self, role: &str, permission: &str) -> bool {
        if let Some(perms) = self.permissions.get(role) {
            perms.contains(&permission.to_string())
        } else {
            false
        }
    }
    
    pub fn add_permission(&mut self, role: &str, permission: &str) {
        self.permissions
            .entry(role.to_string())
            .or_insert_with(Vec::new)
            .push(permission.to_string());
    }
    
    pub fn remove_permission(&mut self, role: &str, permission: &str) {
        if let Some(perms) = self.permissions.get_mut(role) {
            perms.retain(|p| p != permission);
        }
    }
}