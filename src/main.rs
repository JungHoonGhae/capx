mod api;
mod auth;
mod blocks;
mod format;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};
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

    /// Append to today's daily note
    Daily {
        /// Markdown text
        text: String,
        /// Skip timestamp
        #[arg(long)]
        no_timestamp: bool,
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    let token = if let Some(t) = &cli.token {
        t.clone()
    } else {
        auth::get_token()?
    };

    let api = api::Api::new(token);
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

        Commands::Daily { text, no_timestamp } => {
            let space_id = get_space_id(&cli, &api)?;
            api.save_to_daily_note(&space_id, text, *no_timestamp)?;
            if json_mode {
                format::print_json(&serde_json::json!({ "status": "ok" }));
            } else {
                format::print_status("Saved", "daily note updated");
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
    }

    Ok(())
}
