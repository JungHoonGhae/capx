use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use uuid::Uuid;

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()
});

use crate::blocks::{blocks_to_markdown, markdown_to_blocks};
use crate::types::*;

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

const PORTAL_URL: &str = "https://portal.capacities.io";

fn portal_fetch(
    client: &Client,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> Result<Value> {
    let url = format!("{PORTAL_URL}{path}");
    let mut req = match method {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        _ => client.post(&url),
    };

    req = req
        .header("Content-Type", "application/json")
        .header("auth-token", token)
        .header("appversion", "electron-1.58.42-1");

    if let Some(b) = body {
        req = req.json(&b);
    }

    let res = req.send().context("Failed to send request to Portal API")?;
    let status = res.status();
    if !status.is_success() {
        let text = res.text().unwrap_or_default();
        bail!("Portal API error {status}: {text}");
    }

    let text = res.text().unwrap_or_default();
    if text.is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_str(&text).context("Failed to parse API response")
    }
}

pub struct Api {
    client: Client,
    token: String,
}

impl Api {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    // --- Basics ---

    pub fn get_spaces(&self) -> Result<SpacesResponse> {
        let val = portal_fetch(&self.client, "GET", "/basics/spaces", &self.token, None)?;
        Ok(serde_json::from_value(val)?)
    }

    pub fn get_user(&self) -> Result<UserInfo> {
        let val = portal_fetch(&self.client, "GET", "/user", &self.token, None)?;
        Ok(serde_json::from_value(val)?)
    }

    pub fn lookup(&self, query: &str, space_id: &str) -> Result<SearchResponse> {
        let val = portal_fetch(
            &self.client,
            "POST",
            "/basics/lookup",
            &self.token,
            Some(json!({ "searchTerm": query, "spaceId": space_id })),
        )?;
        Ok(serde_json::from_value(val)?)
    }

    pub fn save_weblink(
        &self,
        space_id: &str,
        url: &str,
        title: Option<&str>,
        description: Option<&str>,
        tags: Option<Vec<String>>,
        md_text: Option<&str>,
    ) -> Result<Value> {
        let mut body = json!({ "spaceId": space_id, "url": url });
        if let Some(t) = title {
            body["titleOverwrite"] = json!(t);
        }
        if let Some(d) = description {
            body["descriptionOverwrite"] = json!(d);
        }
        if let Some(t) = tags {
            body["tags"] = json!(t);
        }
        if let Some(m) = md_text {
            body["mdText"] = json!(m);
        }
        portal_fetch(
            &self.client,
            "POST",
            "/basics/save-weblink",
            &self.token,
            Some(body),
        )
    }

    pub fn save_to_daily_note(
        &self,
        space_id: &str,
        md_text: &str,
        no_timestamp: bool,
    ) -> Result<()> {
        let mut body = json!({ "spaceId": space_id, "mdText": md_text });
        if no_timestamp {
            body["noTimeStamp"] = json!(true);
        }
        portal_fetch(
            &self.client,
            "POST",
            "/basics/save-to-daily-note",
            &self.token,
            Some(body),
        )?;
        Ok(())
    }

    /// Find the RootDailyNote component for a given date (YYYY-MM-DD).
    /// Scans space content in batches, returns the first match.
    pub fn get_daily_note(&self, space_id: &str, date: &str) -> Result<Option<Component>> {
        let target_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .context("Invalid date format, expected YYYY-MM-DD")?;

        let space_content = self.get_space_content(space_id)?;
        let elements = space_content.elements.unwrap_or_default();

        for chunk in elements.chunks(50) {
            let ids: Vec<String> = chunk.iter().map(|e| e.id.clone()).collect();
            let result = self.get_content_by_ids(&ids)?;
            for c in result.components.unwrap_or_default() {
                if c.comp_type != "RootDailyNote" {
                    continue;
                }
                if let Some(created) = &c.created_at {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created) {
                        if dt.date_naive() == target_date {
                            return Ok(Some(c));
                        }
                    }
                    // Also try parsing without timezone (some dates may be plain ISO)
                    if let Ok(dt) =
                        chrono::NaiveDateTime::parse_from_str(created, "%Y-%m-%dT%H:%M:%S%.fZ")
                    {
                        if dt.date() == target_date {
                            return Ok(Some(c));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get blocks from a daily note component as markdown.
    pub fn daily_note_to_markdown(component: &Component) -> String {
        let blocks = component
            .data
            .get("blocks")
            .and_then(|b| b.get("RootDailyNote_notes"))
            .and_then(|b| b.as_array());

        match blocks {
            Some(blocks) => blocks_to_markdown(blocks),
            None => String::new(),
        }
    }

    /// Update a daily note's blocks via /content/syncing.
    pub fn sync_daily_note(
        &self,
        space_id: &str,
        component: &Component,
        new_blocks: Vec<Value>,
    ) -> Result<String> {
        let sync_client_id = Uuid::new_v4().to_string();
        let now = now_iso();

        let mut merged = serde_json::to_value(component)?;
        merged["lastUpdated"] = json!(now);

        // Replace the blocks
        if merged["data"]["blocks"].is_object() {
            merged["data"]["blocks"]["RootDailyNote_notes"] = json!(new_blocks);
        } else {
            merged["data"]["blocks"] = json!({ "RootDailyNote_notes": new_blocks });
        }

        let res = portal_fetch(
            &self.client,
            "POST",
            "/content/syncing",
            &self.token,
            Some(json!({
                "syncClientId": sync_client_id,
                "elements": [{ "spaceId": space_id, "content": merged }]
            })),
        )?;

        Ok(extract_sync_status(&res))
    }

    /// Create a task (RootTask) using dynamic property handling.
    /// Properties (status, priority, etc.) are resolved from the RootTask structure definition.
    pub fn save_task(
        &self,
        space_id: &str,
        title: &str,
        md_text: Option<&str>,
        properties: Option<&HashMap<String, Value>>,
        context_ids: Option<&[String]>,
    ) -> Result<(String, String)> {
        // RootTask uses create_entity with type="RootTask", structureId="RootTask"
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let sync_client_id = Uuid::new_v4().to_string();

        // Fetch RootTask structure to get property definitions
        let struct_data = self.get_content_by_ids(&["RootTask".to_string()])?;
        let structure = struct_data.components.and_then(|c| c.into_iter().next());

        let prop_defs: Vec<RawPropertyDefinition> = structure
            .as_ref()
            .and_then(|s| s.data.get("propertyDefinitions"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // Initialize all properties from definitions
        let mut props = json!({
            "title": { "val": title },
            "icon": {},
            "description": {}
        });
        let mut blocks_map: HashMap<String, Vec<Value>> = HashMap::new();

        for def in &prop_defs {
            if ["title", "description", "icon"].contains(&def.id.as_str()) {
                continue;
            }
            match def.data_type.as_str() {
                "blocks" => {
                    props[&def.id] = json!({ "val": def.id });
                    blocks_map.insert(def.id.clone(), vec![]);
                }
                "label" | "entity" => {
                    props[&def.id] = json!({ "val": [] });
                }
                _ => {
                    props[&def.id] = json!({});
                }
            }
        }

        // Apply user properties (status, priority, etc.) using dynamic normalization
        if let Some(user_props) = properties {
            let name_to_def: HashMap<String, &RawPropertyDefinition> = prop_defs
                .iter()
                .filter_map(|d| d.name.as_ref().map(|n| (n.val.to_lowercase(), d)))
                .collect();

            for (key, val) in user_props {
                let (prop_id, prop_def) = resolve_prop_key(key, &prop_defs, &name_to_def);
                if let Some(def) = prop_def {
                    let normalized = normalize_property(def, &prop_id, val, &mut blocks_map);
                    for (k, v) in normalized {
                        props[k] = v;
                    }
                } else {
                    // For built-in RootTask props without definitions (backlinks, etc.)
                    props[&prop_id] = val.clone();
                }
            }
        }

        // Build blocks from markdown body
        if let Some(md) = md_text {
            let blocks_key = prop_defs
                .iter()
                .find(|p| p.data_type == "blocks")
                .map(|p| p.id.clone())
                .unwrap_or_else(|| "RootTask_notes".to_string());
            blocks_map.insert(blocks_key, markdown_to_blocks(md));
        }

        // Find RootTask's RootDatabase
        let db_id = self.find_database_for_type(space_id, "RootTask")?;
        let databases = match &db_id {
            Some(id) => json!([{
                "id": id,
                "link": {
                    "createdAt": now,
                    "data": { "toStructureId": "RootDatabase" },
                    "id": Uuid::new_v4().to_string(),
                    "policies": [],
                    "type": "Database"
                }
            }]),
            None => json!([]),
        };

        let blocks_json: Value = blocks_map
            .into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect::<serde_json::Map<String, Value>>()
            .into();

        let content = json!({
            "id": id,
            "type": "RootTask",
            "structureId": "RootTask",
            "loadingState": "full",
            "deleteRequested": false,
            "databases": databases,
            "policies": [{
                "name": "write",
                "principals": [{
                    "name": "SpaceEditor",
                    "config": { "spaceId": space_id }
                }],
                "principalType": "Role"
            }],
            "lastUpdated": now,
            "createdAt": now,
            "properties": props,
            "data": {
                "blocks": blocks_json,
                "hidePropertySection": false
            },
            "linkNodes": [],
            "local": {}
        });

        let elements = vec![json!({ "spaceId": space_id, "content": content })];

        let res = portal_fetch(
            &self.client,
            "POST",
            "/content/syncing",
            &self.token,
            Some(json!({
                "syncClientId": sync_client_id,
                "elements": elements
            })),
        )?;

        let create_status = extract_sync_status(&res);

        // Add context via LinkToken injection if context_ids provided
        if create_status == "success" {
            if let Some(ctx_ids) = context_ids {
                if !ctx_ids.is_empty() {
                    self.add_context(space_id, &id, ctx_ids)?;
                }
            }
        }

        Ok((id, create_status))
    }

    /// Resolve context references: UUIDs pass through, non-UUIDs are searched.
    /// Returns resolved entity IDs.
    pub fn resolve_context_refs(&self, space_id: &str, refs: &[String]) -> Result<Vec<String>> {
        let mut ids = Vec::new();

        for r in refs {
            if UUID_RE.is_match(&r.to_lowercase()) {
                ids.push(r.clone());
            } else {
                // Search for the entity by name
                let results = self.lookup(r, space_id)?;
                if let Some(first) = results.results.first() {
                    eprintln!("  Resolved \"{}\" → {} ({})", r, first.title, first.id);
                    ids.push(first.id.clone());
                } else {
                    eprintln!("  Warning: no match for \"{}\"", r);
                }
            }
        }

        Ok(ids)
    }

    /// Add context entities to an existing entity by setting the backlinks property.
    /// Add context entities to an object by inserting LinkToken references.
    ///
    /// Context in Capacities works via the backlink index: when entity A references
    /// entity B in its body (via LinkToken), the index adds B to A's "linked nodes"
    /// and A to B's Context. So to make `context_entity_ids` appear in `entity_id`'s
    /// Context, we add LinkTokens referencing `entity_id` to each context entity's body.
    pub fn add_context(
        &self,
        space_id: &str,
        entity_id: &str,
        context_entity_ids: &[String],
    ) -> Result<String> {
        // Fetch the target entity to get its title and structure
        let target_content = self.get_content_by_ids(&[entity_id.to_string()])?;
        let target = target_content
            .components
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| anyhow::anyhow!("Object {entity_id} not found"))?;

        let target_title = target
            .properties
            .get("title")
            .and_then(|t| t.get("val"))
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let target_structure_id = &target.structure_id;

        // Fetch all context entities
        let context_content = self.get_content_by_ids(context_entity_ids)?;
        let context_components = context_content.components.unwrap_or_default();

        let sync_client_id = Uuid::new_v4().to_string();
        let now = now_iso();
        let mut elements = Vec::new();

        for ctx_comp in &context_components {
            // Find the first blocks property to append to
            let prop_defs: Vec<RawPropertyDefinition> = {
                let struct_data =
                    self.get_content_by_ids(std::slice::from_ref(&ctx_comp.structure_id))?;
                struct_data
                    .components
                    .and_then(|c| c.into_iter().next())
                    .and_then(|s| s.data.get("propertyDefinitions").cloned())
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default()
            };

            let blocks_prop = prop_defs.iter().find(|p| p.data_type == "blocks");
            let blocks_key = match blocks_prop {
                Some(p) => p.id.clone(),
                None => continue, // skip entities without a body
            };

            let mut data = ctx_comp.data.clone();
            let blocks_obj = data
                .as_object_mut()
                .and_then(|d| d.get_mut("blocks"))
                .and_then(|b| b.as_object_mut());

            let block_list = match blocks_obj {
                Some(obj) => obj
                    .entry(&blocks_key)
                    .or_insert_with(|| json!([]))
                    .as_array_mut(),
                None => continue,
            };

            let block_list = match block_list {
                Some(l) => l,
                None => continue,
            };

            // Check if a LinkToken referencing entity_id already exists
            let already_linked = block_list.iter().any(|block| {
                block
                    .get("tokens")
                    .and_then(|t| t.as_array())
                    .map(|tokens| {
                        tokens.iter().any(|tok| {
                            tok.get("entity")
                                .and_then(|e| e.get("id"))
                                .and_then(|i| i.as_str())
                                == Some(entity_id)
                        })
                    })
                    .unwrap_or(false)
            });

            if already_linked {
                continue;
            }

            // Build a new TextBlock with a LinkToken referencing the target
            let link_token = json!({
                "id": Uuid::new_v4().to_string(),
                "type": "LinkToken",
                "text": target_title,
                "style": { "bold": false, "italic": false },
                "entity": {
                    "id": entity_id,
                    "link": {
                        "id": Uuid::new_v4().to_string(),
                        "type": "Dependency",
                        "createdAt": now,
                        "data": {
                            "toStructureId": target_structure_id
                        }
                    }
                }
            });

            let new_block = json!({
                "id": Uuid::new_v4().to_string(),
                "type": "TextBlock",
                "tokens": [link_token],
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 }
            });

            block_list.push(new_block);

            let mut merged = serde_json::to_value(ctx_comp)?;
            merged["lastUpdated"] = json!(now);
            merged["data"] = data;

            elements.push(json!({ "spaceId": space_id, "content": merged }));
        }

        if elements.is_empty() {
            return Ok("no_changes".to_string());
        }

        let res = portal_fetch(
            &self.client,
            "POST",
            "/content/syncing",
            &self.token,
            Some(json!({
                "syncClientId": sync_client_id,
                "elements": elements
            })),
        )?;

        let statuses: Vec<&str> = res
            .get("componentReturnObjects")
            .and_then(|arr| arr.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|obj| obj.get("status").and_then(|s| s.as_str()))
                    .collect()
            })
            .unwrap_or_default();

        if statuses.iter().all(|s| *s == "success") {
            Ok("success".to_string())
        } else {
            Ok(format!("partial: {:?}", statuses))
        }
    }

    // --- Content ---

    pub fn get_space_content(&self, space_id: &str) -> Result<SpaceContentResponse> {
        let val = portal_fetch(
            &self.client,
            "POST",
            "/content/space-content",
            &self.token,
            Some(json!({ "spaceId": space_id })),
        )?;
        Ok(serde_json::from_value(val)?)
    }

    pub fn get_content_by_ids(&self, ids: &[String]) -> Result<ContentResponse> {
        let val = portal_fetch(
            &self.client,
            "POST",
            "/content/id-list",
            &self.token,
            Some(json!({ "ids": ids })),
        )?;
        Ok(serde_json::from_value(val)?)
    }

    pub fn get_content_trash(&self, space_id: &str) -> Result<Value> {
        portal_fetch(
            &self.client,
            "GET",
            &format!("/content/trash/{space_id}"),
            &self.token,
            None,
        )
    }

    pub fn duplicate_content(&self, id: &str, space_id: &str) -> Result<Value> {
        portal_fetch(
            &self.client,
            "POST",
            "/content/duplicate",
            &self.token,
            Some(json!({ "id": id, "spaceId": space_id })),
        )
    }

    pub fn undo_delete(&self, id: &str, space_id: &str) -> Result<Value> {
        let sync_client_id = Uuid::new_v4().to_string();
        portal_fetch(
            &self.client,
            "POST",
            "/content/undoDelete",
            &self.token,
            Some(json!({ "id": id, "spaceId": space_id, "syncClientId": sync_client_id })),
        )
    }

    // --- Structures ---

    pub fn get_structures(&self, space_id: &str) -> Result<Vec<StructureInfo>> {
        let space_content = self.get_space_content(space_id)?;
        let elements = space_content.elements.unwrap_or_default();
        if elements.is_empty() {
            return Ok(vec![]);
        }

        let mut structures = Vec::new();

        for chunk in elements.chunks(50) {
            let ids: Vec<String> = chunk.iter().map(|e| e.id.clone()).collect();
            let result = self.get_content_by_ids(&ids)?;
            for c in result.components.unwrap_or_default() {
                if c.comp_type != "RootStructure" {
                    continue;
                }
                let prop_defs: Vec<RawPropertyDefinition> = serde_json::from_value(
                    c.data
                        .get("propertyDefinitions")
                        .cloned()
                        .unwrap_or(json!([])),
                )
                .unwrap_or_default();

                let properties: Vec<StructureProperty> = prop_defs
                    .iter()
                    .map(|p| StructureProperty {
                        id: p.id.clone(),
                        name: p
                            .name
                            .as_ref()
                            .map(|n| n.val.clone())
                            .unwrap_or_else(|| p.id.clone()),
                        data_type: p.data_type.clone(),
                        prop_type: p.prop_type.clone(),
                        is_array: p.is_array,
                        options: p.set.clone(),
                        allowed_structures: p.allowed_structures.clone(),
                    })
                    .collect();

                let title = c
                    .properties
                    .get("title")
                    .and_then(|v| v.get("val"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let plural_name = c
                    .properties
                    .get("pluralName")
                    .and_then(|v| v.get("val"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                structures.push(StructureInfo {
                    id: c.id,
                    title,
                    plural_name,
                    icon: c.properties.get("icon").cloned(),
                    properties,
                });
            }
        }

        Ok(structures)
    }

    pub fn find_structure_by_name(
        &self,
        space_id: &str,
        type_name: &str,
    ) -> Result<Option<StructureInfo>> {
        let structures = self.get_structures(space_id)?;
        let lower = type_name.to_lowercase();
        Ok(structures
            .into_iter()
            .find(|s| s.title.to_lowercase() == lower || s.plural_name.to_lowercase() == lower))
    }

    // --- Space objects summary ---

    pub fn get_space_objects_summary(
        &self,
        space_id: &str,
        filter_type_names: Option<&[String]>,
    ) -> Result<SpaceObjectsSummary> {
        let space_content = self.get_space_content(space_id)?;
        let elements = space_content.elements.unwrap_or_default();
        if elements.is_empty() {
            return Ok(SpaceObjectsSummary {
                elements: vec![],
                summary: HashMap::new(),
                total: 0,
            });
        }

        let mut all_entities: Vec<(String, String, String, String)> = Vec::new(); // (id, title, lastUpdated, structureId)
        let mut structure_name_map: HashMap<String, String> = HashMap::new();

        for chunk in elements.chunks(50) {
            let ids: Vec<String> = chunk.iter().map(|e| e.id.clone()).collect();
            let result = self.get_content_by_ids(&ids)?;
            for c in result.components.unwrap_or_default() {
                if c.comp_type == "RootStructure" {
                    let title = c
                        .properties
                        .get("title")
                        .and_then(|v| v.get("val"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&c.id)
                        .to_string();
                    structure_name_map.insert(c.id, title);
                } else if c.comp_type == "RootEntity" {
                    if let Some(elem) = elements.iter().find(|e| e.id == c.id) {
                        let title = c
                            .properties
                            .get("title")
                            .and_then(|v| v.get("val"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Untitled")
                            .to_string();
                        all_entities.push((c.id, title, elem.last_updated.clone(), c.structure_id));
                    }
                }
            }
        }

        // Resolve unresolved structure IDs
        let unresolved: Vec<String> = all_entities
            .iter()
            .map(|(_, _, _, sid)| sid.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|id| !structure_name_map.contains_key(id))
            .collect();

        if !unresolved.is_empty() {
            let result = self.get_content_by_ids(&unresolved)?;
            for s in result.components.unwrap_or_default() {
                if s.comp_type == "RootStructure" {
                    let title = s
                        .properties
                        .get("title")
                        .and_then(|v| v.get("val"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&s.id)
                        .to_string();
                    structure_name_map.insert(s.id, title);
                }
            }
        }

        let mut summary: HashMap<String, usize> = HashMap::new();
        let result_elements: Vec<SpaceObjectElement> = all_entities
            .iter()
            .map(|(id, title, last_updated, structure_id)| {
                let type_name = structure_name_map
                    .get(structure_id)
                    .cloned()
                    .unwrap_or_else(|| structure_id.clone());
                *summary.entry(type_name.clone()).or_insert(0) += 1;
                SpaceObjectElement {
                    id: id.clone(),
                    title: title.clone(),
                    last_updated: last_updated.clone(),
                    structure_id: structure_id.clone(),
                    type_name,
                }
            })
            .collect();

        let total = result_elements.len();

        let filtered = if let Some(names) = filter_type_names {
            let lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
            result_elements
                .into_iter()
                .filter(|e| lower.contains(&e.type_name.to_lowercase()))
                .collect()
        } else {
            result_elements
        };

        Ok(SpaceObjectsSummary {
            elements: filtered,
            summary,
            total,
        })
    }

    // --- Formatted objects ---

    pub fn get_formatted_objects(&self, ids: &[String]) -> Result<Vec<FormattedObject>> {
        let raw = self.get_content_by_ids(ids)?;
        let components = raw.components.unwrap_or_default();
        let entities: Vec<&Component> = components
            .iter()
            .filter(|c| c.comp_type == "RootEntity")
            .collect();

        if entities.is_empty() {
            return Ok(vec![]);
        }

        // Fetch structure definitions
        let structure_ids: Vec<String> = entities
            .iter()
            .map(|c| c.structure_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut struct_map: HashMap<String, (String, Vec<RawPropertyDefinition>)> = HashMap::new();
        if !structure_ids.is_empty() {
            let struct_data = self.get_content_by_ids(&structure_ids)?;
            for s in struct_data.components.unwrap_or_default() {
                if s.comp_type == "RootStructure" {
                    let title = s
                        .properties
                        .get("title")
                        .and_then(|v| v.get("val"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let prop_defs: Vec<RawPropertyDefinition> = serde_json::from_value(
                        s.data
                            .get("propertyDefinitions")
                            .cloned()
                            .unwrap_or(json!([])),
                    )
                    .unwrap_or_default();
                    struct_map.insert(s.id, (title, prop_defs));
                }
            }
        }

        // Collect entity reference IDs for bulk resolution
        let mut entity_ref_ids: HashSet<String> = HashSet::new();
        for c in &entities {
            if let Some((_, prop_defs)) = struct_map.get(&c.structure_id) {
                for def in prop_defs {
                    if def.data_type == "entity" {
                        if let Some(val) = c.properties.get(&def.id).and_then(|v| v.get("val")) {
                            collect_entity_ids(val, &mut entity_ref_ids);
                        }
                    }
                }
            }
            // Tags
            if let Some(tags) = c
                .properties
                .get("tags")
                .and_then(|v| v.get("val"))
                .and_then(|v| v.as_array())
            {
                for t in tags {
                    if let Some(id) = t.as_str() {
                        entity_ref_ids.insert(id.to_string());
                    }
                }
            }
        }

        // Bulk resolve entity titles
        let mut entity_title_map: HashMap<String, String> = HashMap::new();
        let ref_ids: Vec<String> = entity_ref_ids
            .into_iter()
            .filter(|id| !id.is_empty())
            .collect();
        for chunk in ref_ids.chunks(50) {
            let ids_vec: Vec<String> = chunk.to_vec();
            if let Ok(ref_data) = self.get_content_by_ids(&ids_vec) {
                for r in ref_data.components.unwrap_or_default() {
                    let title = r
                        .properties
                        .get("title")
                        .and_then(|v| v.get("val"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&r.id)
                        .to_string();
                    entity_title_map.insert(r.id, title);
                }
            }
        }

        // Build formatted objects
        let mut result = Vec::new();
        for c in entities {
            let struct_info = struct_map.get(&c.structure_id);
            let (type_name, prop_defs) = struct_info
                .map(|(t, p)| (Some(t.clone()), p.as_slice()))
                .unwrap_or((None, &[]));

            let title = c
                .properties
                .get("title")
                .and_then(|v| v.get("val"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let description = c
                .properties
                .get("description")
                .and_then(|v| v.get("val"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            // Resolve tags
            let tags = c
                .properties
                .get("tags")
                .and_then(|v| v.get("val"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|id| id.as_str())
                        .map(|id| {
                            entity_title_map
                                .get(id)
                                .cloned()
                                .unwrap_or_else(|| id.to_string())
                        })
                        .collect::<Vec<_>>()
                });

            // Build properties
            let mut readable_props: HashMap<String, Value> = HashMap::new();
            let blocks = c.data.get("blocks");
            for def in prop_defs {
                if ["title", "description", "tags"].contains(&def.id.as_str()) {
                    continue;
                }
                let name = def
                    .name
                    .as_ref()
                    .map(|n| n.val.clone())
                    .unwrap_or_else(|| def.id.clone());

                match def.data_type.as_str() {
                    "blocks" => {
                        if let Some(block_arr) = blocks
                            .and_then(|b| b.get(&def.id))
                            .and_then(|v| v.as_array())
                        {
                            if !block_arr.is_empty() {
                                readable_props.insert(name, json!(blocks_to_markdown(block_arr)));
                            }
                        }
                    }
                    "label" => {
                        if let Some(set) = &def.set {
                            let val = c.properties.get(&def.id).and_then(|v| v.get("val"));
                            if let Some(arr) = val.and_then(|v| v.as_array()) {
                                let resolved: Vec<String> = arr
                                    .iter()
                                    .filter_map(|id| id.as_str())
                                    .map(|id| {
                                        set.iter()
                                            .find(|o| o.id == id)
                                            .map(|o| o.text.clone())
                                            .unwrap_or_else(|| id.to_string())
                                    })
                                    .collect();
                                if !resolved.is_empty() {
                                    readable_props.insert(name, json!(resolved));
                                }
                            }
                        }
                    }
                    "entity" => {
                        let val = c.properties.get(&def.id).and_then(|v| v.get("val"));
                        if let Some(val) = val {
                            let resolved = resolve_entity_val(val, &entity_title_map);
                            if !is_empty_val(&resolved) {
                                readable_props.insert(name, resolved);
                            }
                        }
                    }
                    "datetime" | "string" => {
                        if let Some(val) = c.properties.get(&def.id).and_then(|v| v.get("val")) {
                            if !val.is_null() && val.as_str().is_none_or(|s| !s.is_empty()) {
                                readable_props.insert(name, val.clone());
                            }
                        }
                    }
                    _ => {
                        if let Some(val) = c.properties.get(&def.id).and_then(|v| v.get("val")) {
                            if !val.is_null() {
                                readable_props.insert(name, val.clone());
                            }
                        }
                    }
                }
            }

            // Build body from all block properties
            let mut body_parts: Vec<String> = Vec::new();
            if let Some(blocks_obj) = blocks {
                if let Some(obj) = blocks_obj.as_object() {
                    for (_, block_arr) in obj {
                        if let Some(arr) = block_arr.as_array() {
                            if !arr.is_empty() {
                                let md = blocks_to_markdown(arr);
                                if !md.trim().is_empty() {
                                    body_parts.push(md);
                                }
                            }
                        }
                    }
                }
            }

            result.push(FormattedObject {
                id: c.id.clone(),
                obj_type: c.structure_id.clone(),
                type_name,
                title,
                description,
                tags,
                created_at: c.created_at.clone().unwrap_or_default(),
                last_updated: c.last_updated.clone().unwrap_or_default(),
                properties: readable_props,
                body: body_parts.join("\n\n"),
            });
        }

        Ok(result)
    }

    // --- Create / Update / Delete ---

    #[allow(clippy::too_many_arguments)]
    pub fn create_object(
        &self,
        space_id: &str,
        structure_id: &str,
        title: &str,
        description: Option<&str>,
        properties: Option<&HashMap<String, Value>>,
        body_markdown: Option<&str>,
        context_ids: Option<&[String]>,
    ) -> Result<(String, String)> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let sync_client_id = Uuid::new_v4().to_string();

        // Fetch structure definition
        let struct_data = self.get_content_by_ids(&[structure_id.to_string()])?;
        let structure = struct_data.components.and_then(|c| c.into_iter().next());

        let mut props = json!({
            "title": { "val": title },
            "description": description.map(|d| json!({ "val": d })).unwrap_or(json!({})),
            "icon": {},
            "tags": { "val": [] }
        });

        let mut blocks_map: HashMap<String, Vec<Value>> = HashMap::new();

        // Initialize from structure definition
        let prop_defs: Vec<RawPropertyDefinition> = structure
            .as_ref()
            .and_then(|s| s.data.get("propertyDefinitions"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        for def in &prop_defs {
            if ["title", "description", "tags"].contains(&def.id.as_str()) {
                continue;
            }
            match def.data_type.as_str() {
                "blocks" => {
                    props[&def.id] = json!({ "val": def.id });
                    blocks_map.insert(def.id.clone(), vec![]);
                }
                "label" | "entity" => {
                    props[&def.id] = json!({ "val": [] });
                }
                _ => {
                    props[&def.id] = json!({});
                }
            }
        }

        // Merge user properties
        if let Some(user_props) = properties {
            let name_to_def: HashMap<String, &RawPropertyDefinition> = prop_defs
                .iter()
                .filter_map(|d| d.name.as_ref().map(|n| (n.val.to_lowercase(), d)))
                .collect();

            for (key, val) in user_props {
                let (prop_id, prop_def) = resolve_prop_key(key, &prop_defs, &name_to_def);
                if let Some(def) = prop_def {
                    let normalized = normalize_property(def, &prop_id, val, &mut blocks_map);
                    for (k, v) in normalized {
                        props[k] = v;
                    }
                } else {
                    props[&prop_id] = val.clone();
                }
            }
        }

        // Handle bodyMarkdown
        if let Some(body_md) = body_markdown {
            if let Some(first_block_prop) = prop_defs.iter().find(|p| p.data_type == "blocks") {
                blocks_map.insert(first_block_prop.id.clone(), markdown_to_blocks(body_md));
            }
        }

        // Database link
        let db_result = self.find_database_for_structure(space_id, structure_id)?;
        let database_id = db_result.unwrap_or_else(|| Uuid::new_v4().to_string());

        let blocks_json: Value = blocks_map
            .into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect::<serde_json::Map<String, Value>>()
            .into();

        let content = json!({
            "id": id,
            "type": "RootEntity",
            "loadingState": "full",
            "structureId": structure_id,
            "deleteRequested": false,
            "databases": [{
                "id": database_id,
                "link": {
                    "id": Uuid::new_v4().to_string(),
                    "data": { "toStructureId": "RootDatabase" },
                    "type": "Database",
                    "policies": [],
                    "createdAt": now,
                }
            }],
            "policies": [{
                "name": "write",
                "principals": [{
                    "name": "SpaceEditor",
                    "config": { "spaceId": space_id }
                }],
                "principalType": "Role"
            }],
            "lastUpdated": now,
            "createdAt": now,
            "properties": props,
            "data": { "blocks": blocks_json, "hidePropertySection": false },
            "linkNodes": [],
            "local": {}
        });

        let elements = vec![json!({ "spaceId": space_id, "content": content })];

        let res = portal_fetch(
            &self.client,
            "POST",
            "/content/syncing",
            &self.token,
            Some(json!({
                "syncClientId": sync_client_id,
                "elements": elements
            })),
        )?;

        let status = extract_sync_status(&res);

        // Add context via LinkToken injection if context_ids provided
        if status == "success" {
            if let Some(ctx_ids) = context_ids {
                if !ctx_ids.is_empty() {
                    self.add_context(space_id, &id, ctx_ids)?;
                }
            }
        }

        Ok((id, status))
    }

    pub fn update_object(
        &self,
        space_id: &str,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        body_markdown: Option<&str>,
        properties: Option<&HashMap<String, Value>>,
    ) -> Result<String> {
        let existing = self.get_content_by_ids(&[id.to_string()])?;
        let component = existing
            .components
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| anyhow::anyhow!("Object {id} not found"))?;

        let sync_client_id = Uuid::new_v4().to_string();
        let now = now_iso();

        let mut merged_props = component.properties.clone();
        let mut merged_data = component.data.clone();

        if let Some(t) = title {
            merged_props["title"] = json!({ "val": t });
        }
        if let Some(d) = description {
            merged_props["description"] = json!({ "val": d });
        }

        // Fetch structure definition once if needed for properties or body
        let needs_struct =
            (properties.is_some() && !properties.unwrap().is_empty()) || body_markdown.is_some();
        let prop_defs: Vec<RawPropertyDefinition> = if needs_struct {
            let struct_data =
                self.get_content_by_ids(std::slice::from_ref(&component.structure_id))?;
            struct_data
                .components
                .and_then(|c| c.into_iter().next())
                .and_then(|s| s.data.get("propertyDefinitions").cloned())
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Normalize user properties
        if let Some(user_props) = properties {
            if !user_props.is_empty() {
                let name_to_def: HashMap<String, &RawPropertyDefinition> = prop_defs
                    .iter()
                    .filter_map(|d| d.name.as_ref().map(|n| (n.val.to_lowercase(), d)))
                    .collect();

                let mut normalized_blocks: HashMap<String, Vec<Value>> = HashMap::new();
                for (key, val) in user_props {
                    let (prop_id, prop_def) = resolve_prop_key(key, &prop_defs, &name_to_def);
                    if let Some(def) = prop_def {
                        let normalized =
                            normalize_property(def, &prop_id, val, &mut normalized_blocks);
                        for (k, v) in normalized {
                            merged_props[k] = v;
                        }
                    } else {
                        merged_props[&prop_id] = val.clone();
                    }
                }

                // Merge blocks
                if !normalized_blocks.is_empty() {
                    let existing_blocks = merged_data.get("blocks").cloned().unwrap_or(json!({}));
                    let mut blocks_obj = existing_blocks.as_object().cloned().unwrap_or_default();
                    for (k, v) in normalized_blocks {
                        blocks_obj.insert(k, json!(v));
                    }
                    merged_data["blocks"] = Value::Object(blocks_obj);
                }
            }
        }

        // Handle bodyMarkdown
        if let Some(body_md) = body_markdown {
            if let Some(first_block_prop) = prop_defs.iter().find(|p| p.data_type == "blocks") {
                let blocks = markdown_to_blocks(body_md);
                let existing_blocks = merged_data.get("blocks").cloned().unwrap_or(json!({}));
                let mut blocks_obj = existing_blocks.as_object().cloned().unwrap_or_default();
                blocks_obj.insert(first_block_prop.id.clone(), json!(blocks));
                merged_data["blocks"] = Value::Object(blocks_obj);
            }
        }

        let mut merged = json!({});
        // Copy all fields from component
        if let Value::Object(obj) = serde_json::to_value(&component)? {
            for (k, v) in obj {
                merged[&k] = v;
            }
        }
        merged["lastUpdated"] = json!(now);
        merged["properties"] = merged_props;
        merged["data"] = merged_data;

        let elements = vec![json!({ "spaceId": space_id, "content": merged })];

        let res = portal_fetch(
            &self.client,
            "POST",
            "/content/syncing",
            &self.token,
            Some(json!({
                "syncClientId": sync_client_id,
                "elements": elements
            })),
        )?;

        let status = extract_sync_status(&res);

        Ok(status)
    }

    pub fn delete_object(&self, space_id: &str, id: &str) -> Result<String> {
        let existing = self.get_content_by_ids(&[id.to_string()])?;
        let component = existing
            .components
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| anyhow::anyhow!("Object {id} not found"))?;

        let sync_client_id = Uuid::new_v4().to_string();
        let now = now_iso();

        let mut merged = serde_json::to_value(&component)?;
        merged["lastUpdated"] = json!(now);
        merged["deleteRequested"] = json!(true);

        let res = portal_fetch(
            &self.client,
            "POST",
            "/content/syncing",
            &self.token,
            Some(json!({
                "syncClientId": sync_client_id,
                "elements": [{ "spaceId": space_id, "content": merged }]
            })),
        )?;

        let status = extract_sync_status(&res);

        Ok(status)
    }

    // --- Helpers ---

    fn find_database(
        &self,
        space_id: &str,
        predicate: impl Fn(&Component) -> bool,
    ) -> Result<Option<String>> {
        let space_content = self.get_space_content(space_id)?;
        let elements = space_content.elements.unwrap_or_default();
        if elements.is_empty() {
            return Ok(None);
        }

        for chunk in elements.chunks(50) {
            let ids: Vec<String> = chunk.iter().map(|e| e.id.clone()).collect();
            let result = self.get_content_by_ids(&ids)?;
            for c in result.components.unwrap_or_default() {
                if predicate(&c) {
                    if let Some(dbs) = &c.databases {
                        if let Some(first) = dbs.first() {
                            if let Some(db_id) = first.get("id").and_then(|v| v.as_str()) {
                                return Ok(Some(db_id.to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn find_database_for_type(&self, space_id: &str, type_name: &str) -> Result<Option<String>> {
        self.find_database(space_id, |c| c.comp_type == type_name)
    }

    fn find_database_for_structure(
        &self,
        space_id: &str,
        structure_id: &str,
    ) -> Result<Option<String>> {
        self.find_database(space_id, |c| {
            c.structure_id == structure_id && c.comp_type == "RootEntity"
        })
    }
}

// --- Free helpers ---

fn extract_sync_status(res: &Value) -> String {
    res.get("componentReturnObjects")
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.first())
        .and_then(|obj| obj.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn collect_entity_ids(val: &Value, ids: &mut HashSet<String>) {
    if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                ids.insert(s.to_string());
            } else if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                ids.insert(id.to_string());
            }
        }
    } else if let Some(s) = val.as_str() {
        if !s.is_empty() {
            ids.insert(s.to_string());
        }
    }
}

fn resolve_entity_val(val: &Value, title_map: &HashMap<String, String>) -> Value {
    if let Some(arr) = val.as_array() {
        let resolved: Vec<Value> = arr
            .iter()
            .map(|item| {
                let entity_id = if let Some(s) = item.as_str() {
                    s.to_string()
                } else if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    id.to_string()
                } else {
                    item.to_string()
                };
                json!({
                    "id": entity_id,
                    "title": title_map.get(&entity_id).cloned().unwrap_or_else(|| entity_id.clone())
                })
            })
            .collect();
        json!(resolved)
    } else if let Some(s) = val.as_str() {
        if s.is_empty() {
            Value::Null
        } else {
            json!({
                "id": s,
                "title": title_map.get(s).cloned().unwrap_or_else(|| s.to_string())
            })
        }
    } else {
        val.clone()
    }
}

fn is_empty_val(v: &Value) -> bool {
    v.is_null() || (v.is_array() && v.as_array().unwrap().is_empty())
}

fn resolve_prop_key<'a>(
    key: &str,
    prop_defs: &'a [RawPropertyDefinition],
    name_map: &HashMap<String, &'a RawPropertyDefinition>,
) -> (String, Option<&'a RawPropertyDefinition>) {
    let is_uuid = UUID_RE.is_match(&key.to_lowercase());

    if is_uuid {
        let def = prop_defs.iter().find(|d| d.id == key);
        (key.to_string(), def)
    } else if let Some(def) = name_map.get(&key.to_lowercase()) {
        (def.id.clone(), Some(*def))
    } else {
        (key.to_string(), None)
    }
}

fn normalize_property(
    prop_def: &RawPropertyDefinition,
    prop_id: &str,
    raw_val: &Value,
    blocks: &mut HashMap<String, Vec<Value>>,
) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    // Unwrap {val: inner} if pre-wrapped
    let val = if let Some(inner) = raw_val.get("val") {
        inner
    } else {
        raw_val
    };

    match prop_def.data_type.as_str() {
        "blocks" => {
            let md = val.as_str().unwrap_or(&val.to_string()).to_string();
            blocks.insert(prop_id.to_string(), markdown_to_blocks(&md));
            result.insert(prop_id.to_string(), json!({ "val": prop_id }));
        }
        "label" => {
            if let Some(set) = &prop_def.set {
                if let Some(s) = val.as_str() {
                    let lower = s.to_lowercase();
                    let found = set
                        .iter()
                        .find(|o| o.text.to_lowercase() == lower || o.id == s);
                    let id = found.map(|o| o.id.clone()).unwrap_or_else(|| s.to_string());
                    result.insert(prop_id.to_string(), json!({ "val": [id] }));
                } else if let Some(arr) = val.as_array() {
                    result.insert(prop_id.to_string(), json!({ "val": arr }));
                } else {
                    result.insert(prop_id.to_string(), json!({ "val": [] }));
                }
            } else {
                result.insert(prop_id.to_string(), json!({ "val": val }));
            }
        }
        "datetime" => {
            // datetime properties use inline date objects: { dateResolution, startTime }
            if let Some(date_str) = val.as_str() {
                if !date_str.is_empty() {
                    let (start_time, resolution) = parse_date_to_inline(date_str);
                    result.insert(
                        prop_id.to_string(),
                        json!({
                            "val": {
                                "dateResolution": resolution,
                                "startTime": start_time
                            }
                        }),
                    );
                } else {
                    result.insert(prop_id.to_string(), json!({}));
                }
            } else {
                result.insert(prop_id.to_string(), json!({ "val": val }));
            }
        }
        "entity" => {
            let items: Vec<&Value> = if let Some(arr) = val.as_array() {
                arr.iter().collect()
            } else if !val.is_null() {
                vec![val]
            } else {
                vec![]
            };

            let to_structure = prop_def
                .allowed_structures
                .as_ref()
                .and_then(|s| s.first())
                .cloned()
                .unwrap_or_default();

            let linked: Vec<Value> = items
                .iter()
                .map(|item| {
                    if item.get("link").is_some() {
                        (*item).clone()
                    } else {
                        let target_id = item.as_str().unwrap_or(&item.to_string()).to_string();
                        create_entity_link(&target_id, prop_id, &to_structure)
                    }
                })
                .collect();

            result.insert(prop_id.to_string(), json!({ "val": linked }));
        }
        _ => {
            result.insert(prop_id.to_string(), json!({ "val": val }));
        }
    }

    result
}

/// Parse a date string into (startTime, resolution) for inline date format.
/// Supports: "2026-03-23", "2026-03-23T10:00:00Z", "2026-03-23T10:00:00.000Z"
fn parse_date_to_inline(date_str: &str) -> (String, String) {
    if date_str.contains('T') {
        // Full ISO datetime → keep as-is, use "time" resolution
        let normalized = if date_str.ends_with('Z') || date_str.contains('+') {
            date_str.to_string()
        } else {
            format!("{date_str}Z")
        };
        (normalized, "time".to_string())
    } else if date_str.len() == 10 {
        // Date only (2026-03-23) → day resolution
        (format!("{date_str}T00:00:00.000Z"), "day".to_string())
    } else {
        // Fallback
        (format!("{date_str}T00:00:00.000Z"), "day".to_string())
    }
}

fn create_entity_link(target_id: &str, property_id: &str, to_structure_id: &str) -> Value {
    json!({
        "id": target_id,
        "link": {
            "id": Uuid::new_v4().to_string(),
            "data": { "propertyId": property_id, "toStructureId": to_structure_id },
            "type": "Dependency",
            "createdAt": now_iso()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    // --- extract_sync_status ---

    #[test]
    fn extract_sync_status_success() {
        let res = json!({"componentReturnObjects": [{"id": "x", "status": "success"}]});
        assert_eq!(extract_sync_status(&res), "success");
    }

    #[test]
    fn extract_sync_status_unknown_on_empty() {
        let res = json!({"componentReturnObjects": []});
        assert_eq!(extract_sync_status(&res), "unknown");
    }

    #[test]
    fn extract_sync_status_missing_field() {
        let res = json!({"other": "data"});
        assert_eq!(extract_sync_status(&res), "unknown");
    }

    #[test]
    fn extract_sync_status_null() {
        let res = json!(null);
        assert_eq!(extract_sync_status(&res), "unknown");
    }

    // --- collect_entity_ids ---

    #[test]
    fn collect_entity_ids_array_strings() {
        let val = json!(["id1", "id2"]);
        let mut ids = HashSet::new();
        collect_entity_ids(&val, &mut ids);
        assert!(ids.contains("id1"));
        assert!(ids.contains("id2"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn collect_entity_ids_array_objects() {
        let val = json!([{"id": "a"}, {"id": "b"}]);
        let mut ids = HashSet::new();
        collect_entity_ids(&val, &mut ids);
        assert!(ids.contains("a"));
        assert!(ids.contains("b"));
    }

    #[test]
    fn collect_entity_ids_single_string() {
        let val = json!("single-id");
        let mut ids = HashSet::new();
        collect_entity_ids(&val, &mut ids);
        assert!(ids.contains("single-id"));
    }

    #[test]
    fn collect_entity_ids_empty_string() {
        let val = json!("");
        let mut ids = HashSet::new();
        collect_entity_ids(&val, &mut ids);
        assert!(ids.is_empty());
    }

    #[test]
    fn collect_entity_ids_empty_array() {
        let val = json!([]);
        let mut ids = HashSet::new();
        collect_entity_ids(&val, &mut ids);
        assert!(ids.is_empty());
    }

    // --- resolve_entity_val ---

    #[test]
    fn resolve_entity_val_array_mapped() {
        let val = json!(["id1", "id2"]);
        let mut map = HashMap::new();
        map.insert("id1".to_string(), "Title 1".to_string());
        map.insert("id2".to_string(), "Title 2".to_string());
        let result = resolve_entity_val(&val, &map);
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0]["title"], "Title 1");
        assert_eq!(arr[1]["title"], "Title 2");
    }

    #[test]
    fn resolve_entity_val_array_unmapped() {
        let val = json!(["unknown-id"]);
        let map = HashMap::new();
        let result = resolve_entity_val(&val, &map);
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0]["id"], "unknown-id");
        assert_eq!(arr[0]["title"], "unknown-id");
    }

    #[test]
    fn resolve_entity_val_single_string() {
        let val = json!("id1");
        let mut map = HashMap::new();
        map.insert("id1".to_string(), "Title".to_string());
        let result = resolve_entity_val(&val, &map);
        assert_eq!(result["id"], "id1");
        assert_eq!(result["title"], "Title");
    }

    #[test]
    fn resolve_entity_val_empty_string() {
        let val = json!("");
        let map = HashMap::new();
        assert!(resolve_entity_val(&val, &map).is_null());
    }

    #[test]
    fn resolve_entity_val_null() {
        let val = json!(null);
        let map = HashMap::new();
        assert!(resolve_entity_val(&val, &map).is_null());
    }

    // --- is_empty_val ---

    #[test]
    fn is_empty_val_null() {
        assert!(is_empty_val(&json!(null)));
    }

    #[test]
    fn is_empty_val_empty_array() {
        assert!(is_empty_val(&json!([])));
    }

    #[test]
    fn is_empty_val_non_empty_array() {
        assert!(!is_empty_val(&json!([1])));
    }

    #[test]
    fn is_empty_val_string() {
        assert!(!is_empty_val(&json!("hello")));
    }

    // --- resolve_prop_key ---

    fn make_prop_def(id: &str, name: &str, data_type: &str) -> RawPropertyDefinition {
        RawPropertyDefinition {
            id: id.to_string(),
            data_type: data_type.to_string(),
            name: Some(ValWrapper {
                val: name.to_string(),
            }),
            description: None,
            icon: None,
            read_only: false,
            prop_type: String::new(),
            is_array: None,
            allowed_structures: None,
            set: None,
            mode: None,
            constraints: None,
        }
    }

    #[test]
    fn resolve_prop_key_by_uuid() {
        let def = make_prop_def("550e8400-e29b-41d4-a716-446655440000", "Status", "label");
        let defs = vec![def];
        let name_map: HashMap<String, &RawPropertyDefinition> = defs
            .iter()
            .filter_map(|d| d.name.as_ref().map(|n| (n.val.to_lowercase(), d)))
            .collect();
        let (id, found) =
            resolve_prop_key("550e8400-e29b-41d4-a716-446655440000", &defs, &name_map);
        assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
        assert!(found.is_some());
    }

    #[test]
    fn resolve_prop_key_by_name_case_insensitive() {
        let def = make_prop_def("prop-uuid", "Status", "label");
        let defs = vec![def];
        let name_map: HashMap<String, &RawPropertyDefinition> = defs
            .iter()
            .filter_map(|d| d.name.as_ref().map(|n| (n.val.to_lowercase(), d)))
            .collect();
        let (id, found) = resolve_prop_key("STATUS", &defs, &name_map);
        assert_eq!(id, "prop-uuid");
        assert!(found.is_some());
    }

    #[test]
    fn resolve_prop_key_unknown() {
        let defs: Vec<RawPropertyDefinition> = vec![];
        let name_map: HashMap<String, &RawPropertyDefinition> = HashMap::new();
        let (id, found) = resolve_prop_key("unknown", &defs, &name_map);
        assert_eq!(id, "unknown");
        assert!(found.is_none());
    }

    // --- normalize_property ---

    #[test]
    fn normalize_property_label_string_match() {
        let mut def = make_prop_def("status", "Status", "label");
        def.set = Some(vec![
            LabelOption {
                id: "opt-1".to_string(),
                text: "Done".to_string(),
                color: "green".to_string(),
            },
            LabelOption {
                id: "opt-2".to_string(),
                text: "Todo".to_string(),
                color: "red".to_string(),
            },
        ]);
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "status", &json!("done"), &mut blocks);
        let val = result.get("status").unwrap();
        let arr = val["val"].as_array().unwrap();
        assert_eq!(arr[0], "opt-1");
    }

    #[test]
    fn normalize_property_label_array() {
        let mut def = make_prop_def("status", "Status", "label");
        def.set = Some(vec![]);
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "status", &json!(["a", "b"]), &mut blocks);
        let val = result.get("status").unwrap();
        assert_eq!(val["val"], json!(["a", "b"]));
    }

    #[test]
    fn normalize_property_datetime_date_only() {
        let def = make_prop_def("due", "Due", "datetime");
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "due", &json!("2026-03-23"), &mut blocks);
        let val = result.get("due").unwrap();
        assert_eq!(val["val"]["dateResolution"], "day");
        assert_eq!(val["val"]["startTime"], "2026-03-23T00:00:00.000Z");
    }

    #[test]
    fn normalize_property_datetime_full_iso() {
        let def = make_prop_def("due", "Due", "datetime");
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "due", &json!("2026-03-23T10:00:00Z"), &mut blocks);
        let val = result.get("due").unwrap();
        assert_eq!(val["val"]["dateResolution"], "time");
        assert_eq!(val["val"]["startTime"], "2026-03-23T10:00:00Z");
    }

    #[test]
    fn normalize_property_entity() {
        let mut def = make_prop_def("related", "Related", "entity");
        def.allowed_structures = Some(vec!["StructA".to_string()]);
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "related", &json!("target-id"), &mut blocks);
        let val = result.get("related").unwrap();
        let arr = val["val"].as_array().unwrap();
        assert_eq!(arr[0]["id"], "target-id");
        assert!(arr[0]["link"].is_object());
    }

    #[test]
    fn normalize_property_blocks() {
        let def = make_prop_def("notes", "Notes", "blocks");
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "notes", &json!("# Hello\nWorld"), &mut blocks);
        assert!(blocks.contains_key("notes"));
        assert!(!blocks["notes"].is_empty());
        let val = result.get("notes").unwrap();
        assert_eq!(val["val"], "notes");
    }

    #[test]
    fn normalize_property_string_passthrough() {
        let def = make_prop_def("url", "URL", "string");
        let mut blocks = HashMap::new();
        let result = normalize_property(&def, "url", &json!("https://example.com"), &mut blocks);
        let val = result.get("url").unwrap();
        assert_eq!(val["val"], "https://example.com");
    }

    // --- parse_date_to_inline ---

    #[test]
    fn parse_date_to_inline_date_only() {
        let (time, res) = parse_date_to_inline("2026-03-23");
        assert_eq!(time, "2026-03-23T00:00:00.000Z");
        assert_eq!(res, "day");
    }

    #[test]
    fn parse_date_to_inline_full_iso_with_z() {
        let (time, res) = parse_date_to_inline("2026-03-23T10:00:00Z");
        assert_eq!(time, "2026-03-23T10:00:00Z");
        assert_eq!(res, "time");
    }

    #[test]
    fn parse_date_to_inline_iso_without_z() {
        let (time, res) = parse_date_to_inline("2026-03-23T10:00:00");
        assert_eq!(time, "2026-03-23T10:00:00Z");
        assert_eq!(res, "time");
    }

    // --- create_entity_link ---

    #[test]
    fn create_entity_link_structure() {
        let link = create_entity_link("target-123", "prop-1", "StructA");
        assert_eq!(link["id"], "target-123");
        assert_eq!(link["link"]["type"], "Dependency");
        assert_eq!(link["link"]["data"]["propertyId"], "prop-1");
        assert_eq!(link["link"]["data"]["toStructureId"], "StructA");
        assert!(link["link"]["id"].as_str().is_some());
        assert!(link["link"]["createdAt"].as_str().is_some());
    }
}
