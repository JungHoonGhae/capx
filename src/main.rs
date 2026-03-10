mod api;
mod auth;
mod blocks;
mod error;
mod format;
mod types;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "capx", about = "Unofficial CLI for Capacities.io", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Space UUID (or set CAP_SPACE_ID env)
    #[arg(long, global = true)]
    space_id: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Auth token (or set CAP_TOKEN env)
    #[arg(long, global = true)]
    token: Option<String>,

    /// Portal API app version (or set CAP_APPVERSION env)
    #[arg(long, global = true)]
    appversion: Option<String>,

    /// Portal API base URL (or set CAP_PORTAL_URL env)
    #[arg(long, global = true)]
    portal_url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all spaces
    Spaces,

    /// Check authentication status
    Whoami,

    /// Search for content
    Search {
        /// Search query
        query: String,
        /// Max results
        #[arg(long, short, default_value = "20")]
        limit: usize,
    },

    /// List object types (structures)
    Types,

    /// List objects in space
    Ls {
        /// Filter by type name
        #[arg(long, short)]
        r#type: Option<Vec<String>>,
    },

    /// Get object(s) by ID
    Get {
        /// Object UUID(s)
        ids: Vec<String>,
        /// Show raw format
        #[arg(long)]
        raw: bool,
    },

    /// Create an object
    Create {
        /// Type name (e.g., "Page", "Person")
        r#type: String,
        /// Object title
        title: String,
        /// Description
        #[arg(long, short)]
        description: Option<String>,
        /// Body in markdown
        #[arg(long, short)]
        body: Option<String>,
        /// Property key=value pairs
        #[arg(long, short, value_parser = parse_key_val)]
        prop: Vec<(String, String)>,
        /// Context: entity UUIDs or search terms
        #[arg(long)]
        context: Vec<String>,
    },

    /// Update an object
    Update {
        /// Object UUID
        id: String,
        /// New title
        #[arg(long, short)]
        title: Option<String>,
        /// New description
        #[arg(long, short)]
        description: Option<String>,
        /// New body in markdown
        #[arg(long, short)]
        body: Option<String>,
        /// Property key=value pairs
        #[arg(long, short, value_parser = parse_key_val)]
        prop: Vec<(String, String)>,
    },

    /// Delete an object (soft)
    Rm {
        /// Object UUID
        id: String,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },

    /// Restore deleted object
    Undo {
        /// Object UUID
        id: String,
    },

    /// Duplicate an object
    Dup {
        /// Object UUID
        id: String,
    },

    /// List trashed items
    Trash,

    /// Save a weblink
    Link {
        /// URL to save
        url: String,
        /// Override title
        #[arg(long, short)]
        title: Option<String>,
        /// Override description
        #[arg(long, short)]
        description: Option<String>,
        /// Markdown text to attach
        #[arg(long, short)]
        body: Option<String>,
    },

    /// Daily note operations (append, get, delete, set)
    Daily {
        #[command(subcommand)]
        action: Option<DailyAction>,

        /// Markdown text (shorthand for `daily append <text>`)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
    },

    /// Create a task
    Task {
        /// Task title
        title: String,
        /// Task body in markdown
        #[arg(long, short)]
        body: Option<String>,
        /// Property key=value pairs (status, priority, etc.)
        #[arg(long, short, value_parser = parse_key_val)]
        prop: Vec<(String, String)>,
        /// Context: entity UUIDs or search terms (e.g., "인텔리안" or UUID)
        #[arg(long, short)]
        context: Vec<String>,
    },

    /// Add context (backlink) entities to an existing object
    Context {
        /// Object UUID to add context to
        id: String,
        /// Context: entity UUIDs or search terms
        entities: Vec<String>,
    },

    /// Export objects from space
    Export {
        /// Filter by type name
        #[arg(long, short)]
        r#type: Option<Vec<String>>,
        /// Output format: md or json
        #[arg(long, short, default_value = "md")]
        format: String,
        /// Output directory (default: stdout)
        #[arg(long, short)]
        output_dir: Option<String>,
    },

    /// Check API connection and auth status
    Doctor,

    /// Check authentication status and validate token
    Auth,

    /// Edit an object interactively in $EDITOR
    Edit {
        /// Object UUID
        id: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate for (bash, zsh, fish)
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Clone)]
enum DailyAction {
    /// Append text to daily note
    Append {
        /// Markdown text
        text: Vec<String>,
        /// Skip timestamp
        #[arg(long)]
        no_timestamp: bool,
        /// Date (YYYY-MM-DD), default today
        #[arg(long)]
        date: Option<String>,
    },
    /// Show daily note content
    Get {
        /// Date (YYYY-MM-DD), default today
        #[arg(long)]
        date: Option<String>,
        /// Show raw block JSON
        #[arg(long)]
        raw: bool,
    },
    /// Delete blocks from daily note
    Delete {
        /// Delete the last N blocks (default 1)
        #[arg(long, default_value = "1")]
        last: usize,
        /// Delete blocks containing this text
        #[arg(long)]
        marker: Option<String>,
        /// Date (YYYY-MM-DD), default today
        #[arg(long)]
        date: Option<String>,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Replace entire daily note content
    Set {
        /// Markdown content (reads from stdin if omitted)
        body: Option<String>,
        /// Date (YYYY-MM-DD), default today
        #[arg(long)]
        date: Option<String>,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
}

fn build_props(prop: &[(String, String)]) -> Option<HashMap<String, serde_json::Value>> {
    if prop.is_empty() {
        None
    } else {
        Some(
            prop.iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect(),
        )
    }
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=VALUE: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_val_valid() {
        let (k, v) = parse_key_val("status=done").unwrap();
        assert_eq!(k, "status");
        assert_eq!(v, "done");
    }

    #[test]
    fn parse_key_val_no_equals() {
        assert!(parse_key_val("noequalssign").is_err());
    }

    #[test]
    fn parse_key_val_value_with_equals() {
        let (k, v) = parse_key_val("url=https://example.com?a=1").unwrap();
        assert_eq!(k, "url");
        assert_eq!(v, "https://example.com?a=1");
    }

    #[test]
    fn parse_key_val_empty_value() {
        let (k, v) = parse_key_val("key=").unwrap();
        assert_eq!(k, "key");
        assert_eq!(v, "");
    }

    #[test]
    fn build_props_empty() {
        assert!(build_props(&[]).is_none());
    }

    #[test]
    fn build_props_non_empty() {
        let pairs = vec![
            ("key1".to_string(), "val1".to_string()),
            ("key2".to_string(), "val2".to_string()),
        ];
        let result = build_props(&pairs).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result["key1"],
            serde_json::Value::String("val1".to_string())
        );
        assert_eq!(
            result["key2"],
            serde_json::Value::String("val2".to_string())
        );
    }
}

fn get_space_id(cli: &Cli, api: &api::Api) -> Result<String> {
    if let Some(id) = &cli.space_id {
        return Ok(id.clone());
    }
    if let Ok(id) = std::env::var("CAP_SPACE_ID") {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    // Auto-detect: use first space
    let spaces = api.get_spaces()?;
    if let Some(first) = spaces.spaces.first() {
        Ok(first.id.clone())
    } else {
        anyhow::bail!("No spaces found. Use --space-id or set CAP_SPACE_ID.")
    }
}

fn resolve_date(date: Option<&str>) -> String {
    match date {
        Some(d) => d.to_string(),
        None => chrono::Local::now().format("%Y-%m-%d").to_string(),
    }
}

fn backup_daily_note(component: &types::Component, date: &str) -> Result<()> {
    let backup_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("capx")
        .join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("daily_{date}_{ts}.json");
    let path = backup_dir.join(&filename);

    let blocks = component
        .data
        .get("blocks")
        .and_then(|b| b.get("RootDailyNote_notes"));
    let backup = serde_json::json!({
        "id": component.id,
        "date": date,
        "blocks": blocks,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&backup)?)?;
    eprintln!("  Backup saved to {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let token = if let Some(t) = &cli.token {
        t.clone()
    } else {
        auth::get_token()?
    };

    let appversion = cli.appversion.clone().or_else(|| {
        std::env::var("CAP_APPVERSION")
            .ok()
            .filter(|s| !s.is_empty())
    });
    let portal_url = cli.portal_url.clone().or_else(|| {
        std::env::var("CAP_PORTAL_URL")
            .ok()
            .filter(|s| !s.is_empty())
    });
    let api = api::Api::new(token, appversion, portal_url);
    let json_mode = cli.json;

    match &cli.command {
        Commands::Spaces => {
            let data = api.get_spaces()?;
            if json_mode {
                format::print_json(&data);
            } else {
                format::print_spaces(&data.spaces);
            }
        }

        Commands::Whoami => {
            let data = api.get_user()?;
            if json_mode {
                format::print_json(&data);
            } else {
                format::print_user(&data);
            }
        }

        Commands::Search { query, limit } => {
            let space_id = get_space_id(&cli, &api)?;
            let data = api.lookup(query, &space_id)?;
            let results: Vec<_> = data.results.into_iter().take(*limit).collect();
            if json_mode {
                format::print_json(&results);
            } else {
                format::print_search_results(&results);
            }
        }

        Commands::Types => {
            let space_id = get_space_id(&cli, &api)?;
            let data = api.get_structures(&space_id)?;
            if json_mode {
                format::print_json(&data);
            } else {
                format::print_structures(&data);
            }
        }

        Commands::Ls { r#type } => {
            let space_id = get_space_id(&cli, &api)?;
            let filter = r#type.as_deref();
            let data = api.get_space_objects_summary(&space_id, filter)?;
            if json_mode {
                format::print_json(&data);
            } else {
                format::print_objects_summary(&data);
            }
        }

        Commands::Get { ids, raw } => {
            if ids.is_empty() {
                anyhow::bail!("At least one ID required.");
            }
            let id_strings: Vec<String> = ids.clone();
            if *raw {
                let data = api.get_content_by_ids(&id_strings)?;
                format::print_json(&data);
            } else {
                let data = api.get_formatted_objects(&id_strings)?;
                if json_mode {
                    format::print_json(&data);
                } else {
                    format::print_formatted_objects(&data);
                }
            }
        }

        Commands::Create {
            r#type,
            title,
            description,
            body,
            prop,
            context,
        } => {
            let space_id = get_space_id(&cli, &api)?;
            let structure = api.find_structure_by_name(&space_id, r#type)?;
            let structure = structure.ok_or_else(|| {
                anyhow::anyhow!("Type \"{}\" not found. Use `capx types` to list.", r#type)
            })?;

            let props = build_props(prop);

            let context_ids = if context.is_empty() {
                None
            } else {
                let resolved = api.resolve_context_refs(&space_id, context)?;
                Some(resolved)
            };

            let (id, status) = api.create_object(
                &space_id,
                &structure.id,
                title,
                description.as_deref(),
                props.as_ref(),
                body.as_deref(),
                context_ids.as_deref(),
            )?;

            if json_mode {
                format::print_json(&serde_json::json!({ "id": id, "status": status }));
            } else {
                format::print_created(&id, &status);
            }
        }

        Commands::Update {
            id,
            title,
            description,
            body,
            prop,
        } => {
            let space_id = get_space_id(&cli, &api)?;
            let props = build_props(prop);

            let status = api.update_object(
                &space_id,
                id,
                title.as_deref(),
                description.as_deref(),
                body.as_deref(),
                props.as_ref(),
            )?;

            if json_mode {
                format::print_json(&serde_json::json!({ "id": id, "status": status }));
            } else {
                format::print_status("Updated", &status);
            }
        }

        Commands::Rm { id, yes } => {
            if !yes {
                eprint!("Delete {}? [y/N] ", id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }
            let space_id = get_space_id(&cli, &api)?;
            let status = api.delete_object(&space_id, id)?;
            if json_mode {
                format::print_json(&serde_json::json!({ "id": id, "status": status }));
            } else {
                format::print_status("Deleted", &status);
            }
        }

        Commands::Undo { id } => {
            let space_id = get_space_id(&cli, &api)?;
            let data = api.undo_delete(id, &space_id)?;
            if json_mode {
                format::print_json(&data);
            } else {
                format::print_status("Restored", "ok");
            }
        }

        Commands::Dup { id } => {
            let space_id = get_space_id(&cli, &api)?;
            let data = api.duplicate_content(id, &space_id)?;
            if json_mode {
                format::print_json(&data);
            } else {
                let new_id = data.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                format::print_created(new_id, "duplicated");
            }
        }

        Commands::Trash => {
            let space_id = get_space_id(&cli, &api)?;
            let data = api.get_content_trash(&space_id)?;
            if json_mode {
                format::print_json(&data);
            } else {
                format::print_trash(&data);
            }
        }

        Commands::Link {
            url,
            title,
            description,
            body,
        } => {
            let space_id = get_space_id(&cli, &api)?;
            let data = api.save_weblink(
                &space_id,
                url,
                title.as_deref(),
                description.as_deref(),
                None,
                body.as_deref(),
            )?;
            if json_mode {
                format::print_json(&data);
            } else {
                let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                format::print_created(id, "saved");
            }
        }

        Commands::Daily { action, text } => {
            let space_id = get_space_id(&cli, &api)?;

            // Determine which action to take
            let resolved_action = if let Some(a) = action {
                a.clone()
            } else if !text.is_empty() {
                // Backward compat: `capx daily "some text"` → append
                DailyAction::Append {
                    text: text.clone(),
                    no_timestamp: false,
                    date: None,
                }
            } else {
                // No action and no text → show today's note
                DailyAction::Get {
                    date: None,
                    raw: false,
                }
            };

            match resolved_action {
                DailyAction::Append {
                    text: append_text,
                    no_timestamp,
                    date,
                } => {
                    let md = append_text.join(" ");
                    if md.is_empty() {
                        anyhow::bail!("Text required for append.");
                    }
                    if date.is_some() {
                        // For non-today dates, we need to fetch + modify blocks directly
                        let date_str = resolve_date(date.as_deref());
                        let component = api
                            .get_daily_note(&space_id, &date_str)?
                            .ok_or_else(|| anyhow::anyhow!("No daily note found for {date_str}"))?;

                        let mut blocks = component
                            .data
                            .get("blocks")
                            .and_then(|b| b.get("RootDailyNote_notes"))
                            .and_then(|b| b.as_array())
                            .cloned()
                            .unwrap_or_default();

                        // Add timestamp block if needed
                        if !no_timestamp {
                            let ts = chrono::Local::now().format("%H:%M").to_string();
                            let ts_blocks = crate::blocks::markdown_to_blocks(&format!("**{ts}**"));
                            blocks.extend(ts_blocks);
                        }

                        let new_blocks = crate::blocks::markdown_to_blocks(&md);
                        blocks.extend(new_blocks);

                        let status = api.sync_daily_note(&space_id, &component, blocks)?;
                        if json_mode {
                            format::print_json(&serde_json::json!({ "status": status }));
                        } else {
                            format::print_status(
                                "Saved",
                                &format!("daily note {date_str} updated"),
                            );
                        }
                    } else {
                        // Today: use the simple append endpoint
                        api.save_to_daily_note(&space_id, &md, no_timestamp)?;
                        if json_mode {
                            format::print_json(&serde_json::json!({ "status": "ok" }));
                        } else {
                            format::print_status("Saved", "daily note updated");
                        }
                    }
                }

                DailyAction::Get { date, raw } => {
                    let date_str = resolve_date(date.as_deref());
                    let component = api
                        .get_daily_note(&space_id, &date_str)?
                        .ok_or_else(|| anyhow::anyhow!("No daily note found for {date_str}"))?;

                    if raw {
                        let blocks = component
                            .data
                            .get("blocks")
                            .and_then(|b| b.get("RootDailyNote_notes"));
                        format::print_json(&blocks);
                    } else if json_mode {
                        let md = api::Api::daily_note_to_markdown(&component);
                        format::print_json(&serde_json::json!({
                            "id": component.id,
                            "date": date_str,
                            "body": md,
                        }));
                    } else {
                        let md = api::Api::daily_note_to_markdown(&component);
                        format::print_daily_note(&date_str, &component.id, &md);
                    }
                }

                DailyAction::Delete {
                    last,
                    marker,
                    date,
                    yes,
                } => {
                    let date_str = resolve_date(date.as_deref());
                    let component = api
                        .get_daily_note(&space_id, &date_str)?
                        .ok_or_else(|| anyhow::anyhow!("No daily note found for {date_str}"))?;

                    let blocks = component
                        .data
                        .get("blocks")
                        .and_then(|b| b.get("RootDailyNote_notes"))
                        .and_then(|b| b.as_array())
                        .cloned()
                        .unwrap_or_default();

                    if blocks.is_empty() {
                        anyhow::bail!("Daily note for {date_str} has no blocks to delete.");
                    }

                    let new_blocks = if let Some(ref marker_text) = marker {
                        // Delete blocks containing the marker text
                        let filtered: Vec<serde_json::Value> = blocks
                            .into_iter()
                            .filter(|b| {
                                let md = crate::blocks::blocks_to_markdown(std::slice::from_ref(b));
                                !md.contains(marker_text)
                            })
                            .collect();
                        filtered
                    } else {
                        // Delete last N blocks
                        let keep = blocks.len().saturating_sub(last);
                        blocks[..keep].to_vec()
                    };

                    let removed = component
                        .data
                        .get("blocks")
                        .and_then(|b| b.get("RootDailyNote_notes"))
                        .and_then(|b| b.as_array())
                        .map(|b| b.len())
                        .unwrap_or(0)
                        - new_blocks.len();

                    if removed == 0 {
                        if json_mode {
                            format::print_json(&serde_json::json!({ "status": "no_match" }));
                        } else {
                            println!("  No matching blocks found.");
                        }
                        return Ok(());
                    }

                    if !yes {
                        eprint!("Delete {removed} block(s) from daily note {date_str}? [y/N] ");
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("Cancelled.");
                            return Ok(());
                        }
                    }

                    // Backup before delete
                    backup_daily_note(&component, &date_str)?;

                    let status = api.sync_daily_note(&space_id, &component, new_blocks)?;
                    if json_mode {
                        format::print_json(
                            &serde_json::json!({ "status": status, "removed": removed }),
                        );
                    } else {
                        format::print_status(
                            "Deleted",
                            &format!("{removed} block(s) from {date_str}"),
                        );
                    }
                }

                DailyAction::Set { body, date, yes } => {
                    let date_str = resolve_date(date.as_deref());
                    let component = api
                        .get_daily_note(&space_id, &date_str)?
                        .ok_or_else(|| anyhow::anyhow!("No daily note found for {date_str}"))?;

                    let md = match body {
                        Some(ref b) => b.clone(),
                        None => {
                            // Read from stdin
                            eprintln!("Reading markdown from stdin (Ctrl-D to end)...");
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                            buf
                        }
                    };

                    if !yes {
                        eprint!("Replace entire daily note for {date_str}? [y/N] ");
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("Cancelled.");
                            return Ok(());
                        }
                    }

                    // Backup before overwrite
                    backup_daily_note(&component, &date_str)?;

                    let new_blocks = crate::blocks::markdown_to_blocks(&md);
                    let status = api.sync_daily_note(&space_id, &component, new_blocks)?;
                    if json_mode {
                        format::print_json(&serde_json::json!({ "status": status }));
                    } else {
                        format::print_status("Set", &format!("daily note {date_str} replaced"));
                    }
                }
            }
        }

        Commands::Task {
            title,
            body,
            prop,
            context,
        } => {
            let space_id = get_space_id(&cli, &api)?;
            let props = build_props(prop);

            let context_ids = if context.is_empty() {
                None
            } else {
                let resolved = api.resolve_context_refs(&space_id, context)?;
                Some(resolved)
            };

            let (id, status_result) = api.save_task(
                &space_id,
                title,
                body.as_deref(),
                props.as_ref(),
                context_ids.as_deref(),
            )?;
            if json_mode {
                format::print_json(&serde_json::json!({ "id": id, "status": status_result }));
            } else {
                format::print_created(&id, &status_result);
            }
        }

        Commands::Context { id, entities } => {
            if entities.is_empty() {
                anyhow::bail!("At least one context entity (UUID or search term) required.");
            }
            let space_id = get_space_id(&cli, &api)?;
            let resolved = api.resolve_context_refs(&space_id, entities)?;
            if resolved.is_empty() {
                anyhow::bail!("No entities resolved from the given references.");
            }
            let status = api.add_context(&space_id, id, &resolved)?;
            if json_mode {
                format::print_json(&serde_json::json!({ "id": id, "status": status }));
            } else {
                format::print_status("Context added", &status);
            }
        }

        Commands::Export {
            r#type,
            format: fmt,
            output_dir,
        } => {
            let space_id = get_space_id(&cli, &api)?;
            let filter = r#type.as_deref();
            let summary = api.get_space_objects_summary(&space_id, filter)?;

            if summary.elements.is_empty() {
                if json_mode {
                    format::print_json(&serde_json::json!([]));
                } else {
                    eprintln!("No objects to export.");
                }
                return Ok(());
            }

            let ids: Vec<String> = summary.elements.iter().map(|e| e.id.clone()).collect();
            let objects = api.get_formatted_objects(&ids)?;

            if let Some(dir) = output_dir {
                std::fs::create_dir_all(dir)?;
                for obj in &objects {
                    let safe_title: String = obj
                        .title
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    let ext = if fmt == "json" { "json" } else { "md" };
                    let filename = format!("{safe_title}.{ext}");
                    let path = std::path::Path::new(&dir).join(&filename);

                    if fmt == "json" {
                        let content = serde_json::to_string_pretty(obj)?;
                        std::fs::write(&path, content)?;
                    } else {
                        let mut content = format!("# {}\n\n", obj.title);
                        if let Some(desc) = &obj.description {
                            if !desc.is_empty() {
                                content.push_str(&format!("> {desc}\n\n"));
                            }
                        }
                        content.push_str(&obj.body);
                        std::fs::write(&path, content)?;
                    }
                }
                eprintln!("Exported {} objects to {dir}/", objects.len());
            } else if fmt == "json" || json_mode {
                format::print_json(&objects);
            } else {
                for (i, obj) in objects.iter().enumerate() {
                    if i > 0 {
                        println!("{}", "─".repeat(60));
                    }
                    println!("# {}", obj.title);
                    if let Some(desc) = &obj.description {
                        if !desc.is_empty() {
                            println!("> {desc}");
                        }
                    }
                    println!();
                    println!("{}", obj.body);
                    println!();
                }
            }
        }

        Commands::Doctor => {
            eprintln!("Checking capx configuration...");
            eprintln!();

            // Check auth
            eprint!("  Auth:       ");
            match api.get_user() {
                Ok(user) => eprintln!("OK ({})", user.email),
                Err(e) => eprintln!("FAIL ({})", e),
            }

            // Check spaces
            eprint!("  Spaces:     ");
            match api.get_spaces() {
                Ok(data) => eprintln!("OK ({} spaces)", data.spaces.len()),
                Err(e) => eprintln!("FAIL ({})", e),
            }

            // Check appversion
            eprintln!("  Appversion: {}", api.appversion());
            eprintln!("  Portal URL: {}", api.portal_url());

            // Check space access
            eprint!("  Space:      ");
            match get_space_id(&cli, &api) {
                Ok(id) => match api.get_structures(&id) {
                    Ok(structures) => {
                        eprintln!("OK ({} types in {})", structures.len(), id);
                    }
                    Err(e) => eprintln!("FAIL ({})", e),
                },
                Err(e) => eprintln!("FAIL ({})", e),
            }
            eprintln!();
            eprintln!("Done.");
        }

        Commands::Auth => match api.get_user() {
            Ok(user) => {
                if json_mode {
                    format::print_json(&serde_json::json!({
                        "authenticated": true,
                        "id": user.id,
                        "email": user.email,
                    }));
                } else {
                    format::print_status("Authenticated", &user.email);
                }
            }
            Err(e) => {
                if json_mode {
                    format::print_json(&serde_json::json!({
                        "authenticated": false,
                        "error": e.to_string(),
                    }));
                } else {
                    anyhow::bail!("Authentication failed: {e}");
                }
            }
        },

        Commands::Edit { id } => {
            let space_id = get_space_id(&cli, &api)?;

            // Fetch current object
            let objects = api.get_formatted_objects(std::slice::from_ref(id))?;
            let obj = objects
                .first()
                .ok_or_else(|| anyhow::anyhow!("Object {id} not found"))?;

            // Write to temp file
            let mut md = format!("# {}\n\n", obj.title);
            if let Some(desc) = &obj.description {
                if !desc.is_empty() {
                    md.push_str(&format!("> {desc}\n\n"));
                }
            }
            md.push_str(&obj.body);

            let tmp_path = std::env::temp_dir().join(format!("capx-edit-{id}.md"));
            std::fs::write(&tmp_path, &md)?;

            // Open in editor
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            let status = std::process::Command::new(&editor)
                .arg(&tmp_path)
                .status()?;

            if !status.success() {
                let _ = std::fs::remove_file(&tmp_path);
                anyhow::bail!("Editor exited with non-zero status");
            }

            // Read back and check for changes
            let new_md = std::fs::read_to_string(&tmp_path)?;
            let _ = std::fs::remove_file(&tmp_path);

            if new_md == md {
                if json_mode {
                    format::print_json(&serde_json::json!({ "status": "no_changes" }));
                } else {
                    println!("  No changes detected.");
                }
                return Ok(());
            }

            // Parse the edited markdown: extract title from first heading, description from blockquote
            let mut new_title: Option<String> = None;
            let mut new_desc: Option<String> = None;
            let mut body_lines = Vec::new();
            let mut past_header = false;

            for line in new_md.lines() {
                if !past_header {
                    if let Some(t) = line.strip_prefix("# ") {
                        new_title = Some(t.to_string());
                        continue;
                    }
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(d) = line.strip_prefix("> ") {
                        if new_desc.is_none() {
                            new_desc = Some(d.to_string());
                            continue;
                        }
                    }
                    past_header = true;
                }
                body_lines.push(line);
            }

            let new_body = body_lines.join("\n");
            let update_status = api.update_object(
                &space_id,
                id,
                new_title.as_deref(),
                new_desc.as_deref(),
                Some(&new_body),
                None,
            )?;

            if json_mode {
                format::print_json(&serde_json::json!({ "id": id, "status": update_status }));
            } else {
                format::print_status("Updated", &update_status);
            }
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "capx", &mut std::io::stdout());
        }
    }

    Ok(())
}
