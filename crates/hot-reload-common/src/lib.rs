use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialData {
    pub resources_path: String,
    pub resources: HashMap<String, Vec<String>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    pub resource_name: String,
    pub change_type: ChangeType,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    FileModified,
    FileAdded,
    FileRemoved,
}
