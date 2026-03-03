use regex::Regex;
use serde_json::{json, Value};
use uuid::Uuid;

// --- Blocks → Markdown ---

fn tokens_to_markdown(tokens: &[Value]) -> String {
    tokens
        .iter()
        .map(|t| {
            let token_type = t["type"].as_str().unwrap_or("");
            match token_type {
                "TextToken" => {
                    let text = t["text"].as_str().unwrap_or("");
                    let bold = t["style"]["bold"].as_bool().unwrap_or(false);
                    let italic = t["style"]["italic"].as_bool().unwrap_or(false);
                    if bold && italic {
                        format!("***{text}***")
                    } else if bold {
                        format!("**{text}**")
                    } else if italic {
                        format!("*{text}*")
                    } else {
                        text.to_string()
                    }
                }
                "CodeToken" => {
                    let code = t["code"]
                        .as_str()
                        .or_else(|| t["text"].as_str())
                        .unwrap_or("");
                    format!("`{code}`")
                }
                "LinkToken" => {
                    let text = t["text"]
                        .as_str()
                        .or_else(|| t["url"].as_str())
                        .unwrap_or("");
                    let url = t["url"].as_str().unwrap_or("");
                    format!("[{text}]({url})")
                }
                _ => t["text"].as_str().unwrap_or("").to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn block_to_markdown(block: &Value) -> String {
    let block_type = block["type"].as_str().unwrap_or("");
    match block_type {
        "TextBlock" => {
            let tokens = block["tokens"].as_array();
            let text = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            let h = block["hierarchy"]["key"].as_str().unwrap_or("Base");
            match h {
                "H1" => format!("# {text}"),
                "H2" => format!("## {text}"),
                "H3" => format!("### {text}"),
                "H4" => format!("#### {text}"),
                _ => text,
            }
        }
        "CodeBlock" => {
            let lang = block["language"].as_str().unwrap_or("");
            let code = block["code"].as_str().unwrap_or_else(|| {
                block["tokens"]
                    .as_array()
                    .map(|t| tokens_to_markdown(t))
                    .unwrap_or_default()
                    .leak()
            });
            format!("```{lang}\n{code}\n```")
        }
        "EntityBlock" => {
            let tokens = block["tokens"].as_array();
            if let Some(tokens) = tokens {
                if let Some(entity) = tokens.iter().find(|t| t["entityTitle"].is_string()) {
                    let title = entity["entityTitle"].as_str().unwrap_or("");
                    return format!("[[{title}]]");
                }
                return tokens
                    .iter()
                    .map(|t| {
                        t["text"]
                            .as_str()
                            .or_else(|| t["entityTitle"].as_str())
                            .unwrap_or("")
                    })
                    .collect::<Vec<_>>()
                    .join("");
            }
            String::new()
        }
        "HorizontalLineBlock" => "---".to_string(),
        "SimpleTableBlock" => {
            let cols = block["columns"].as_array();
            if let Some(cols) = cols {
                if cols.is_empty() {
                    return String::new();
                }
                let max_rows = cols
                    .iter()
                    .map(|c| c["cells"].as_array().map_or(0, |cells| cells.len()))
                    .max()
                    .unwrap_or(0);
                if max_rows == 0 {
                    return String::new();
                }
                let mut rows: Vec<Vec<String>> = Vec::new();
                for r in 0..max_rows {
                    let row: Vec<String> = cols
                        .iter()
                        .map(|c| {
                            c["cells"]
                                .as_array()
                                .and_then(|cells| cells.get(r))
                                .and_then(|cell| cell["tokens"].as_array())
                                .map(|t| tokens_to_markdown(t))
                                .unwrap_or_default()
                        })
                        .collect();
                    rows.push(row);
                }
                let header = format!("| {} |", rows[0].join(" | "));
                let sep = format!("| {} |", rows[0].iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
                let body: Vec<String> = rows[1..]
                    .iter()
                    .map(|r| format!("| {} |", r.join(" | ")))
                    .collect();
                let mut parts = vec![header, sep];
                parts.extend(body);
                parts.join("\n")
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

pub fn blocks_to_markdown(blocks: &[Value]) -> String {
    blocks
        .iter()
        .map(|b| block_to_markdown(b))
        .collect::<Vec<_>>()
        .join("\n")
}

// --- Markdown → Blocks ---

fn parse_inline_tokens(text: &str) -> Vec<Value> {
    let mut tokens = Vec::new();
    let pattern =
        Regex::new(r"(\*\*\*(.+?)\*\*\*|\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`|\[([^\]]+)\]\(([^)]+)\))")
            .unwrap();

    let mut last_index = 0;
    for cap in pattern.captures_iter(text) {
        let m = cap.get(0).unwrap();
        if m.start() > last_index {
            tokens.push(json!({
                "type": "TextToken",
                "id": Uuid::new_v4().to_string(),
                "text": &text[last_index..m.start()],
                "style": { "bold": false, "italic": false }
            }));
        }

        if let Some(bold_italic) = cap.get(2) {
            tokens.push(json!({
                "type": "TextToken",
                "id": Uuid::new_v4().to_string(),
                "text": bold_italic.as_str(),
                "style": { "bold": true, "italic": true }
            }));
        } else if let Some(bold) = cap.get(3) {
            tokens.push(json!({
                "type": "TextToken",
                "id": Uuid::new_v4().to_string(),
                "text": bold.as_str(),
                "style": { "bold": true, "italic": false }
            }));
        } else if let Some(italic) = cap.get(4) {
            tokens.push(json!({
                "type": "TextToken",
                "id": Uuid::new_v4().to_string(),
                "text": italic.as_str(),
                "style": { "bold": false, "italic": true }
            }));
        } else if let Some(code) = cap.get(5) {
            tokens.push(json!({
                "type": "CodeToken",
                "id": Uuid::new_v4().to_string(),
                "code": code.as_str()
            }));
        } else if let Some(link_text) = cap.get(6) {
            let url = cap.get(7).map(|m| m.as_str()).unwrap_or("");
            tokens.push(json!({
                "type": "LinkToken",
                "id": Uuid::new_v4().to_string(),
                "text": link_text.as_str(),
                "url": url
            }));
        }

        last_index = m.end();
    }

    if last_index < text.len() {
        tokens.push(json!({
            "type": "TextToken",
            "id": Uuid::new_v4().to_string(),
            "text": &text[last_index..],
            "style": { "bold": false, "italic": false }
        }));
    }

    if tokens.is_empty() {
        tokens.push(json!({
            "type": "TextToken",
            "id": Uuid::new_v4().to_string(),
            "text": "",
            "style": { "bold": false, "italic": false }
        }));
    }

    tokens
}

pub fn markdown_to_blocks(markdown: &str) -> Vec<Value> {
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    let fence_re = Regex::new(r"^```(\w*)$").unwrap();
    let hr_re = Regex::new(r"^(-{3,}|\*{3,}|_{3,})$").unwrap();
    let heading_re = Regex::new(r"^(#{1,4})\s+(.*)$").unwrap();

    while i < lines.len() {
        let line = lines[i];

        // Code block
        if let Some(cap) = fence_re.captures(line) {
            let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            i += 1; // skip closing fence
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "CodeBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "language": lang,
                "code": code_lines.join("\n"),
                "tokens": []
            }));
            continue;
        }

        // Horizontal rule
        if hr_re.is_match(line) {
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "HorizontalLineBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "tokens": []
            }));
            i += 1;
            continue;
        }

        // Heading
        if let Some(cap) = heading_re.captures(line) {
            let level = cap.get(1).unwrap().as_str().len();
            let text = cap.get(2).unwrap().as_str();
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "TextBlock",
                "blocks": [],
                "hierarchy": { "key": format!("H{level}"), "val": 0 },
                "tokens": parse_inline_tokens(text)
            }));
            i += 1;
            continue;
        }

        // Regular text
        blocks.push(json!({
            "id": Uuid::new_v4().to_string(),
            "type": "TextBlock",
            "blocks": [],
            "hierarchy": { "key": "Base", "val": 0 },
            "tokens": parse_inline_tokens(line)
        }));
        i += 1;
    }

    blocks
}
