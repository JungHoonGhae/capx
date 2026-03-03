use colored::*;
use serde::Serialize;
use serde_json::Value;

use crate::types::*;

pub fn print_json<T: Serialize>(data: &T) {
    println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
}

pub fn print_spaces(spaces: &[Space]) {
    if spaces.is_empty() {
        println!("{}", "No spaces found.".dimmed());
        return;
    }
    for s in spaces {
        println!("  {} {}", s.id.dimmed(), s.title.bold());
    }
}

pub fn print_user(user: &UserInfo) {
    println!("  {} {}", "ID:".dimmed(), user.id);
    println!("  {} {}", "Email:".dimmed(), user.email);
    for role in &user.roles {
        println!("  {} {}", "Role:".dimmed(), role.name);
    }
}

pub fn print_search_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("{}", "No results.".dimmed());
        return;
    }
    for r in results {
        print!("  {} {}", r.id.dimmed(), r.title.bold());
        if let Some(s) = &r.snippet {
            if !s.is_empty() {
                print!(" — {}", s.dimmed());
            }
        }
        println!();
    }
}

pub fn print_structures(structures: &[StructureInfo]) {
    if structures.is_empty() {
        println!("{}", "No types found.".dimmed());
        return;
    }
    for s in structures {
        println!("  {} {}", s.id.dimmed(), s.title.bold());
        for p in &s.properties {
            let extra = match p.data_type.as_str() {
                "label" => {
                    if let Some(opts) = &p.options {
                        let names: Vec<&str> = opts.iter().map(|o| o.text.as_str()).collect();
                        format!(" [{}]", names.join(", "))
                    } else {
                        String::new()
                    }
                }
                "entity" => {
                    if let Some(allowed) = &p.allowed_structures {
                        format!(" → {}", allowed.join(", "))
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            };
            println!(
                "    {} {} {}{}",
                p.id.dimmed(),
                p.name,
                format!("({})", p.data_type).dimmed(),
                extra.dimmed()
            );
        }
    }
}

pub fn print_objects_summary(summary: &SpaceObjectsSummary) {
    println!("  {} {}", "Total:".dimmed(), summary.total);
    println!();
    let mut sorted: Vec<_> = summary.summary.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (type_name, count) in &sorted {
        println!("  {:>4}  {}", count.to_string().bold(), type_name);
    }
    if !summary.elements.is_empty() {
        println!();
        for e in &summary.elements {
            println!("  {} {} {}", e.id.dimmed(), e.type_name.blue(), e.last_updated.dimmed());
        }
    }
}

pub fn print_formatted_objects(objects: &[FormattedObject]) {
    if objects.is_empty() {
        println!("{}", "No objects found.".dimmed());
        return;
    }
    for (i, obj) in objects.iter().enumerate() {
        if i > 0 {
            println!("{}", "─".repeat(60).dimmed());
        }
        let type_label = obj.type_name.as_deref().unwrap_or(&obj.obj_type);
        println!("  {} {}", type_label.blue(), obj.title.bold());
        println!("  {} {}", "ID:".dimmed(), obj.id.dimmed());

        if let Some(desc) = &obj.description {
            if !desc.is_empty() {
                println!("  {} {}", "Desc:".dimmed(), desc);
            }
        }
        if let Some(tags) = &obj.tags {
            if !tags.is_empty() {
                println!("  {} {}", "Tags:".dimmed(), tags.join(", "));
            }
        }

        if !obj.properties.is_empty() {
            for (key, val) in &obj.properties {
                let display = format_prop_value(val);
                if !display.is_empty() {
                    println!("  {} {}", format!("{key}:").dimmed(), display);
                }
            }
        }

        if !obj.body.is_empty() {
            println!();
            for line in obj.body.lines() {
                println!("  {line}");
            }
        }
        println!();
    }
}

fn format_prop_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|v| {
                    if let Some(title) = v.get("title").and_then(|t| t.as_str()) {
                        title.to_string()
                    } else if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        v.to_string()
                    }
                })
                .collect();
            items.join(", ")
        }
        Value::Object(obj) => {
            if let Some(title) = obj.get("title").and_then(|t| t.as_str()) {
                title.to_string()
            } else {
                serde_json::to_string(val).unwrap_or_default()
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
    }
}

pub fn print_created(id: &str, status: &str) {
    println!("  {} {}", "Created:".green(), id);
    println!("  {} {}", "Status:".dimmed(), status);
}

pub fn print_status(action: &str, status: &str) {
    println!("  {} {}", format!("{action}:").green(), status);
}
