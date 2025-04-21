use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub assigned_to: Option<String>,
    pub comments: Vec<TicketComment>,
    pub attachments: Vec<TicketAttachment>,
    pub category: TicketCategory,
    pub tags: Vec<String>,
    pub due_date: Option<DateTime<Utc>>, //Added from original code
    pub resolution: Option<String>, //Added from original code
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketStatus {
    Open,
    InProgress,
    Pending,
    Resolved,
    Closed,
    //Reopened, //Removed from original code - not present in edited code.
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketCategory {
    Access,
    Hardware,
    Software,
    Network,
    Security,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketComment {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub is_internal: bool, //Added from original code
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAttachment {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Clone)]
pub struct TicketsManager {
    tickets: Arc<Mutex<HashMap<Uuid, Ticket>>>,
}

impl TicketsManager {
    pub fn new() -> Self {
        Self {
            tickets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_ticket(&self, 
                      title: String, 
                      description: String, 
                      priority: TicketPriority, 
                      created_by: String,
                      category: TicketCategory,
                      tags: Vec<String>,
                      due_date: Option<DateTime<Utc>>) -> Result<Uuid> { //Added due_date
        let id = Uuid::new_v4();
        let now = Utc::now();

        let ticket = Ticket {
            id,
            title,
            description,
            status: TicketStatus::Open,
            priority,
            created_at: now,
            updated_at: now,
            created_by,
            assigned_to: None,
            comments: Vec::new(),
            attachments: Vec::new(),
            category,
            tags,
            due_date, //Added due_date
            resolution: None, //Added resolution
        };

        match self.tickets.lock() {
            Ok(mut tickets) => {
                tickets.insert(id, ticket);
                Ok(id)
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }

    pub fn update_ticket(&self, 
                      id: Uuid, 
                      title: Option<String>, 
                      description: Option<String>, 
                      status: Option<TicketStatus>,
                      priority: Option<TicketPriority>,
                      assigned_to: Option<Option<String>>,
                      category: Option<TicketCategory>,
                      tags: Option<Vec<String>>,
                      resolution: Option<Option<String>>, //Added resolution
                      due_date: Option<Option<DateTime<Utc>>>) -> Result<()> { //Added due_date
        match self.tickets.lock() {
            Ok(mut tickets) => {
                let ticket = tickets.get_mut(&id)
                    .ok_or_else(|| anyhow!("Ticket not found: {}", id))?;

                if let Some(title) = title {
                    ticket.title = title;
                }

                if let Some(description) = description {
                    ticket.description = description;
                }

                if let Some(status) = status {
                    ticket.status = status;
                }

                if let Some(priority) = priority {
                    ticket.priority = priority;
                }

                if let Some(assigned_to) = assigned_to {
                    ticket.assigned_to = assigned_to.into_iter().flatten(); // Correctly flatten Option<Option<T>>
                }

                if let Some(category) = category {
                    ticket.category = category;
                }

                if let Some(tags) = tags {
                    ticket.tags = tags;
                }

                if let Some(resolution) = resolution {
                    ticket.resolution = resolution.into_iter().flatten(); // Correctly flatten Option<Option<T>>
                }

                if let Some(due_date) = due_date {
                    ticket.due_date = due_date.into_iter().flatten(); // Correctly flatten Option<Option<DateTime<Utc>>>
                }

                ticket.updated_at = Utc::now();

                Ok(())
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }

    pub fn add_comment(&self, ticket_id: Uuid, content: String, created_by: String, is_internal: bool) -> Result<Uuid> { //Added is_internal
        match self.tickets.lock() {
            Ok(mut tickets) => {
                let ticket = tickets.get_mut(&ticket_id)
                    .ok_or_else(|| anyhow!("Ticket not found: {}", ticket_id))?;

                let comment_id = Uuid::new_v4();
                let comment = TicketComment {
                    id: comment_id,
                    ticket_id,
                    content,
                    created_at: Utc::now(),
                    created_by,
                    is_internal, //Added is_internal
                };

                ticket.comments.push(comment);
                ticket.updated_at = Utc::now();

                Ok(comment_id)
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }

    pub fn add_attachment(&self, 
                       ticket_id: Uuid, 
                       filename: String, 
                       content_type: String, 
                       size: usize, 
                       created_by: String) -> Result<Uuid> {
        match self.tickets.lock() {
            Ok(mut tickets) => {
                let ticket = tickets.get_mut(&ticket_id)
                    .ok_or_else(|| anyhow!("Ticket not found: {}", ticket_id))?;

                let attachment_id = Uuid::new_v4();
                let attachment = TicketAttachment {
                    id: attachment_id,
                    ticket_id,
                    filename,
                    content_type,
                    size,
                    created_at: Utc::now(),
                    created_by,
                };

                ticket.attachments.push(attachment);
                ticket.updated_at = Utc::now();

                Ok(attachment_id)
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }

    pub fn get_ticket(&self, id: Uuid) -> Result<Ticket> {
        match self.tickets.lock() {
            Ok(tickets) => {
                tickets.get(&id)
                    .cloned()
                    .ok_or_else(|| anyhow!("Ticket not found: {}", id))
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }

    pub fn get_all_tickets(&self) -> Result<Vec<Ticket>> {
        match self.tickets.lock() {
            Ok(tickets) => {
                Ok(tickets.values().cloned().collect())
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }

    pub fn delete_ticket(&self, id: Uuid) -> Result<()> {
        match self.tickets.lock() {
            Ok(mut tickets) => {
                if tickets.remove(&id).is_none() {
                    return Err(anyhow!("Ticket not found: {}", id));
                }
                Ok(())
            },
            Err(_) => Err(anyhow!("Failed to acquire lock on tickets")),
        }
    }
}

//The rest of the original code is removed because it's replaced by TicketsManager.