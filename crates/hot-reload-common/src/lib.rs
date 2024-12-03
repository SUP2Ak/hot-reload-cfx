use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    FileModified,
    FileAdded,
    FileRemoved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    pub resource_name: String,
    pub change_type: ChangeType,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTree {
    pub folders: HashMap<String, FileTree>,
    pub files: Vec<String>,
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            folders: HashMap::new(),
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCategory {
    pub categories: HashMap<String, ResourceCategory>,
    pub resources: HashMap<String, FileTree>,
}

impl ResourceCategory {
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
            resources: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialData {
    pub resources_path: String,
    pub categories: ResourceCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResponse {
    Success,
    Failed(String),
}