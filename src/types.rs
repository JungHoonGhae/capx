use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Space {
    pub id: String,
    pub title: String,
    pub icon: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpacesResponse {
    pub spaces: Vec<Space>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub roles: Vec<UserRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRole {
    pub name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub id: String,
    #[serde(rename = "structureId")]
    pub structure_id: String,
    pub title: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LabelOption {
    pub id: String,
    pub text: String,
    pub color: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawPropertyDefinition {
    pub id: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    pub name: Option<ValWrapper>,
    pub description: Option<ValWrapper>,
    pub icon: Option<serde_json::Value>,
    #[serde(rename = "readOnly", default)]
    pub read_only: bool,
    #[serde(rename = "type", default)]
    pub prop_type: String,
    #[serde(rename = "isArray")]
    pub is_array: Option<bool>,
    #[serde(rename = "allowedStructures")]
    pub allowed_structures: Option<Vec<String>>,
    pub set: Option<Vec<LabelOption>>,
    pub mode: Option<String>,
    pub constraints: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValWrapper {
    pub val: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureInfo {
    pub id: String,
    pub title: String,
    #[serde(rename = "pluralName")]
    pub plural_name: String,
    pub icon: Option<serde_json::Value>,
    pub properties: Vec<StructureProperty>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureProperty {
    pub id: String,
    pub name: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    #[serde(rename = "isArray", skip_serializing_if = "Option::is_none")]
    pub is_array: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<LabelOption>>,
    #[serde(rename = "allowedStructures", skip_serializing_if = "Option::is_none")]
    pub allowed_structures: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub id: String,
    #[serde(rename = "type")]
    pub comp_type: String,
    #[serde(rename = "structureId", default)]
    pub structure_id: String,
    #[serde(default)]
    pub properties: serde_json::Value,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default)]
    pub databases: Option<Vec<serde_json::Value>>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
    #[serde(rename = "deleteRequested")]
    pub delete_requested: Option<bool>,
    #[serde(rename = "loadingState")]
    pub loading_state: Option<String>,
    pub policies: Option<Vec<serde_json::Value>>,
    #[serde(rename = "linkNodes")]
    pub link_nodes: Option<Vec<serde_json::Value>>,
    pub local: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentResponse {
    pub components: Option<Vec<Component>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceContentElement {
    pub id: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceContentResponse {
    pub elements: Option<Vec<SpaceContentElement>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    #[serde(rename = "componentReturnObjects")]
    pub component_return_objects: Option<Vec<SyncReturnObject>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncReturnObject {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormattedObject {
    pub id: String,
    #[serde(rename = "type")]
    pub obj_type: String,
    #[serde(rename = "typeName", skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceObjectsSummary {
    pub elements: Vec<SpaceObjectElement>,
    pub summary: HashMap<String, usize>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceObjectElement {
    pub id: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    #[serde(rename = "structureId")]
    pub structure_id: String,
    #[serde(rename = "typeName")]
    pub type_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveWeblinkResult {
    #[serde(rename = "spaceId")]
    pub space_id: String,
    pub id: String,
    #[serde(rename = "structureId")]
    pub structure_id: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
}
