use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context, anyhow};
use tracing::{info, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub is_approved: bool,
    pub approved_by: Option<String>,
    pub category: ScriptCategory,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScriptCategory {
    System,
    Network,
    Security,
    UserManagement,
    Maintenance,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionResult {
    pub id: Uuid,
    pub script_id: Uuid,
    pub executed_at: DateTime<Utc>,
    pub executed_by: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub struct ScriptsManager {
    scripts_dir: PathBuf,
    scripts: HashMap<Uuid, Script>,
    execution_results: Vec<ScriptExecutionResult>,
}

impl ScriptsManager {
    pub fn new(scripts_dir: &str) -> Result<Self> {
        let scripts_dir = PathBuf::from(scripts_dir);

        // Create the scripts directory if it doesn't exist
        if !scripts_dir.exists() {
            fs::create_dir_all(&scripts_dir)
                .context(format!("Failed to create scripts directory: {:?}", scripts_dir))?;
            info!("Created scripts directory: {:?}", scripts_dir);
        }

        let mut manager = Self {
            scripts_dir,
            scripts: HashMap::new(),
            execution_results: Vec::new(),
        };

        manager.load_scripts()?;

        Ok(manager)
    }

    fn load_scripts(&mut self) -> Result<()> {
        let scripts_dir = &self.scripts_dir;

        if !scripts_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(scripts_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                match self.load_script(&path) {
                    Ok(script) => {
                        info!("Loaded script: {} ({})", script.name, script.id);
                        self.scripts.insert(script.id, script);
                    },
                    Err(e) => {
                        error!("Failed to load script {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn load_script(&self, path: &Path) -> Result<Script> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let script: Script = serde_json::from_str(&contents)?;
        Ok(script)
    }

    fn save_script(&self, script: &Script) -> Result<()> {
        let file_path = self.scripts_dir.join(format!("{}.json", script.id));
        let json = serde_json::to_string_pretty(script)?;

        let mut file = File::create(file_path)?;
        file.write_all(json.as_bytes())?;

        Ok(())
    }

    pub fn create_script(&mut self, 
                     name: String, 
                     description: String, 
                     content: String, 
                     created_by: String,
                     category: ScriptCategory,
                     tags: Vec<String>) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let script = Script {
            id,
            name,
            description,
            content,
            created_at: now,
            updated_at: now,
            created_by,
            is_approved: false,
            approved_by: None,
            category,
            tags,
        };

        self.save_script(&script)?;
        self.scripts.insert(id, script);

        Ok(id)
    }

    pub fn update_script(&mut self, 
                      id: Uuid, 
                      name: Option<String>, 
                      description: Option<String>, 
                      content: Option<String>,
                      category: Option<ScriptCategory>,
                      tags: Option<Vec<String>>) -> Result<()> {
        // Clone the script first so we don't hold a mutable borrow when calling save_script
        let mut script_clone = {
            let script = self.scripts.get(&id)
                .ok_or_else(|| anyhow!("Script not found: {}", id))?;
            script.clone()
        };

        if let Some(name) = name {
            script_clone.name = name;
        }

        if let Some(description) = description {
            script_clone.description = description;
        }

        if let Some(content) = content {
            script_clone.content = content;
            // When the content changes, approval is reset
            script_clone.is_approved = false;
            script_clone.approved_by = None;
        }

        if let Some(category) = category {
            script_clone.category = category;
        }

        if let Some(tags) = tags {
            script_clone.tags = tags;
        }

        script_clone.updated_at = Utc::now();

        // Save the cloned script and update in-memory storage
        self.save_script(&script_clone)?;
        self.scripts.insert(id, script_clone);

        Ok(())
    }

    pub fn delete_script(&mut self, id: Uuid) -> Result<()> {
        if !self.scripts.contains_key(&id) {
            return Err(anyhow!("Script not found: {}", id));
        }

        let file_path = self.scripts_dir.join(format!("{}.json", id));
        fs::remove_file(file_path)?;

        self.scripts.remove(&id);

        Ok(())
    }

    pub fn approve_script(&mut self, id: Uuid, approved_by: String) -> Result<()> {
        // Clone the script first so we don't hold a mutable borrow when calling save_script
        let mut script_clone = {
            let script = self.scripts.get(&id)
                .ok_or_else(|| anyhow!("Script not found: {}", id))?;
            script.clone()
        };

        script_clone.is_approved = true;
        script_clone.approved_by = Some(approved_by);
        script_clone.updated_at = Utc::now();

        // Save the cloned script and update in-memory storage
        self.save_script(&script_clone)?;
        self.scripts.insert(id, script_clone);

        Ok(())
    }

    pub fn execute_script(&mut self, id: Uuid, executed_by: String) -> Result<ScriptExecutionResult> {
        let script = self.scripts.get(&id)
            .ok_or_else(|| anyhow!("Script not found: {}", id))?;

        if !script.is_approved {
            return Err(anyhow!("Cannot execute unapproved script"));
        }

        info!("Executing script: {} ({})", script.name, script.id);

        let start_time = std::time::Instant::now();
        let execution_id = Uuid::new_v4();

        // Save script to a temporary file
        let temp_script_path = self.scripts_dir.join(format!("temp_{}.ps1", execution_id));
        let mut temp_script = File::create(&temp_script_path)?;
        temp_script.write_all(script.content.as_bytes())?;
        temp_script.flush()?;

        // Execute the script
        let output = Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&temp_script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let duration = start_time.elapsed().as_millis() as u64;

        // Remove temporary file
        if temp_script_path.exists() {
            if let Err(e) = fs::remove_file(&temp_script_path) {
                warn!("Failed to remove temporary script file: {}", e);
            }
        }

        let result = match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let success = output.status.success();
                let error = if !stderr.is_empty() { Some(stderr) } else { None };

                if success {
                    info!("Script execution successful: {} ({})", script.name, script.id);
                } else {
                    error!("Script execution failed: {} ({}): {}", 
                          script.name, script.id, error.clone().unwrap_or_default());
                }

                ScriptExecutionResult {
                    id: execution_id,
                    script_id: id,
                    executed_at: Utc::now(),
                    executed_by,
                    success,
                    output: stdout,
                    error,
                    duration_ms: duration,
                }
            },
            Err(e) => {
                let error_message = format!("Failed to execute script: {}", e);
                error!("{}", error_message);

                ScriptExecutionResult {
                    id: execution_id,
                    script_id: id,
                    executed_at: Utc::now(),
                    executed_by,
                    success: false,
                    output: String::new(),
                    error: Some(error_message),
                    duration_ms: duration,
                }
            }
        };

        self.execution_results.push(result.clone());

        Ok(result)
    }

    pub fn get_script(&self, id: Uuid) -> Option<Script> {
        self.scripts.get(&id).cloned()
    }

    pub fn get_all_scripts(&self) -> Vec<Script> {
        self.scripts.values().cloned().collect()
    }

    pub fn get_execution_results(&self, script_id: Option<Uuid>) -> Vec<ScriptExecutionResult> {
        match script_id {
            Some(id) => self.execution_results.iter()
                            .filter(|r| r.script_id == id)
                            .cloned()
                            .collect(),
            None => self.execution_results.clone(),
        }
    }
}


pub async fn start(config: &Config, _storage: impl Send + Sync + 'static) -> Result<ScriptsManager> {
    let scripts_dir = config.scripts.repository_path.clone();
    let repository = ScriptsManager::new(&scripts_dir)?;
    info!("Script management module started with {} scripts", repository.scripts.len());
    Ok(repository)
}

// Placeholder for Config struct (replace with your actual Config struct)
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub scripts: ScriptsConfig,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptsConfig {
    pub repository_path: String,
    // Add other config fields as needed
}