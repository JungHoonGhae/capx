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
            println!(
                "  {} {} {} {}",
                e.id.dimmed(),
                e.type_name.blue(),
                e.title.bold(),
                e.last_updated.dimmed()
            );
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

pub fn print_trash(data: &Value) {
    let items = data.as_array();
    match items {
        Some(arr) if !arr.is_empty() => {
            for item in arr {
                let id = item["id"].as_str().unwrap_or("?");
                let title = item["properties"]["title"]["val"]
                    .as_str()
                    .unwrap_or("Untitled");
                let structure = item["structureId"].as_str().unwrap_or("");
                println!("  {} {} {}", id.dimmed(), title.bold(), structure.blue());
            }
        }
        _ => println!("{}", "Trash is empty.".dimmed()),
    }
}

pub fn print_daily_note(date: &str, id: &str, body: &str) {
    println!("  {} {}", "Daily Note".blue(), date.bold());
    println!("  {} {}", "ID:".dimmed(), id.dimmed());
    if body.is_empty() {
        println!("\n  {}", "(empty)".dimmed());
    } else {
        println!();
        for line in body.lines() {
            println!("  {line}");
        }
    }
    println!();
}

pub fn print_status(action: &str, status: &str) {
    println!("  {} {}", format!("{action}:").green(), status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_prop_value_string() {
        let val = json!("hello");
        assert_eq!(format_prop_value(&val), "hello");
    }

    #[test]
    fn format_prop_value_array_with_titles() {
        let val = json!([{"title": "Tag1"}, {"title": "Tag2"}]);
        assert_eq!(format_prop_value(&val), "Tag1, Tag2");
    }

    #[test]
    fn format_prop_value_array_strings() {
        let val = json!(["a", "b", "c"]);
        assert_eq!(format_prop_value(&val), "a, b, c");
    }

    #[test]
    fn format_prop_value_object_with_title() {
        let val = json!({"title": "MyObj", "id": "123"});
        assert_eq!(format_prop_value(&val), "MyObj");
    }

    #[test]
    fn format_prop_value_object_no_title() {
        let val = json!({"id": "123"});
        let result = format_prop_value(&val);
        assert!(result.contains("123"));
    }

    #[test]
    fn format_prop_value_bool() {
        assert_eq!(format_prop_value(&json!(true)), "true");
        assert_eq!(format_prop_value(&json!(false)), "false");
    }

    #[test]
    fn format_prop_value_number() {
        assert_eq!(format_prop_value(&json!(42)), "42");
        assert_eq!(format_prop_value(&json!(3.14)), "3.14");
    }

    #[test]
    fn format_prop_value_null() {
        assert_eq!(format_prop_value(&json!(null)), "");
    }

    #[test]
    fn format_prop_value_mixed_array() {
        let val = json!([{"title": "Named"}, "plain", 42]);
        let result = format_prop_value(&val);
        assert!(result.contains("Named"));
        assert!(result.contains("plain"));
        assert!(result.contains("42"));
    }
}
