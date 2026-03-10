use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;
use uuid::Uuid;

static INLINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\*\*\*(.+?)\*\*\*|\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`|\[([^\]]+)\]\(([^)]+)\))")
        .unwrap()
});
static FENCE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^```(\w*)$").unwrap());
static HR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(-{3,}|\*{3,}|_{3,})$").unwrap());
static HEADING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(#{1,4})\s+(.*)$").unwrap());
static CHECKLIST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- \[([ xX])\]\s+(.*)$").unwrap());
static BULLET_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-*+]\s+(.*)$").unwrap());
static NUMBERED_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+\.\s+(.*)$").unwrap());
static QUOTE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^>\s?(.*)$").unwrap());
static CALLOUT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^>\s*\[!(\w+)\]\s*(.*)$").unwrap());
static WIKILINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[\[([^\]|]+)(?:\|([^\]]+))?\]\]$").unwrap());

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
            let code = match block["code"].as_str() {
                Some(s) => s.to_string(),
                None => block["tokens"]
                    .as_array()
                    .map(|t| tokens_to_markdown(t))
                    .unwrap_or_default(),
            };
            format!("```{lang}\n{code}\n```")
        }
        "EntityBlock" => {
            let tokens = block["tokens"].as_array();
            if let Some(tokens) = tokens {
                if let Some(entity) = tokens.iter().find(|t| t["entityTitle"].is_string()) {
                    let title = entity["entityTitle"].as_str().unwrap_or("");
                    let uuid = entity
                        .get("entity")
                        .and_then(|e| e.get("id"))
                        .and_then(|id| id.as_str());
                    return match uuid {
                        Some(id) => format!("[[{title}|{id}]]"),
                        None => format!("[[{title}]]"),
                    };
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
        "ChecklistBlock" => {
            let tokens = block["tokens"].as_array();
            let text = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            let checked = block["checked"].as_bool().unwrap_or(false);
            if checked {
                format!("- [x] {text}")
            } else {
                format!("- [ ] {text}")
            }
        }
        "BulletListBlock" => {
            let tokens = block["tokens"].as_array();
            let text = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            format!("- {text}")
        }
        "NumberedListBlock" => {
            let tokens = block["tokens"].as_array();
            let text = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            let num = block["number"].as_u64().unwrap_or(1);
            format!("{num}. {text}")
        }
        "QuoteBlock" => {
            let tokens = block["tokens"].as_array();
            let text = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            format!("> {text}")
        }
        "CalloutBlock" => {
            let tokens = block["tokens"].as_array();
            let text = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            let callout_type = block["calloutType"].as_str().unwrap_or("NOTE");
            format!("> [!{callout_type}] {text}")
        }
        "ToggleBlock" => {
            let tokens = block["tokens"].as_array();
            let title = tokens.map(|t| tokens_to_markdown(t)).unwrap_or_default();
            let children = block["blocks"].as_array();
            let content = children.map(|c| blocks_to_markdown(c)).unwrap_or_default();
            format!("<details><summary>{title}</summary>{content}</details>")
        }
        "ImageBlock" => {
            let url = block["url"].as_str().unwrap_or("");
            let alt = block["alt"]
                .as_str()
                .or_else(|| block["filename"].as_str())
                .unwrap_or("");
            if url.is_empty() {
                let filename = block["filename"].as_str().unwrap_or("image");
                format!("[Image: {filename}]")
            } else {
                format!("![{alt}]({url})")
            }
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
                let sep = format!(
                    "| {} |",
                    rows[0]
                        .iter()
                        .map(|_| "---")
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
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
        other => format!("<!-- unsupported: {other} -->"),
    }
}

pub fn blocks_to_markdown(blocks: &[Value]) -> String {
    blocks
        .iter()
        .map(block_to_markdown)
        .collect::<Vec<_>>()
        .join("\n")
}

// --- Markdown → Blocks ---

fn parse_inline_tokens(text: &str) -> Vec<Value> {
    let mut tokens = Vec::new();

    let mut last_index = 0;
    for cap in INLINE_RE.captures_iter(text) {
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

    while i < lines.len() {
        let line = lines[i];

        // Code block
        if let Some(cap) = FENCE_RE.captures(line) {
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
        if HR_RE.is_match(line) {
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
        if let Some(cap) = HEADING_RE.captures(line) {
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

        // Checklist: - [x] or - [ ]
        if let Some(cap) = CHECKLIST_RE.captures(line) {
            let checked = cap.get(1).unwrap().as_str() != " ";
            let text = cap.get(2).unwrap().as_str();
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "ChecklistBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "checked": checked,
                "tokens": parse_inline_tokens(text)
            }));
            i += 1;
            continue;
        }

        // Callout: > [!NOTE] text (must be before quote)
        if let Some(cap) = CALLOUT_RE.captures(line) {
            let callout_type = cap.get(1).unwrap().as_str();
            let text = cap.get(2).unwrap().as_str();
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "CalloutBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "calloutType": callout_type,
                "tokens": parse_inline_tokens(text)
            }));
            i += 1;
            continue;
        }

        // Quote: > text
        if let Some(cap) = QUOTE_RE.captures(line) {
            let text = cap.get(1).unwrap().as_str();
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "QuoteBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "tokens": parse_inline_tokens(text)
            }));
            i += 1;
            continue;
        }

        // Numbered list: 1. text
        if let Some(cap) = NUMBERED_RE.captures(line) {
            let text = cap.get(1).unwrap().as_str();
            let num = line
                .split('.')
                .next()
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(1);
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "NumberedListBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "number": num,
                "tokens": parse_inline_tokens(text)
            }));
            i += 1;
            continue;
        }

        // Bullet list: - text (after checklist check to avoid conflict)
        if let Some(cap) = BULLET_RE.captures(line) {
            let text = cap.get(1).unwrap().as_str();
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "BulletListBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "tokens": parse_inline_tokens(text)
            }));
            i += 1;
            continue;
        }

        // Wiki-link: [[Name]] or [[Name|uuid]]
        if let Some(cap) = WIKILINK_RE.captures(line) {
            let title = cap.get(1).unwrap().as_str();
            let uuid = cap.get(2).map(|m| m.as_str());
            let mut token = json!({
                "entityTitle": title,
                "type": "LinkToken",
                "id": Uuid::new_v4().to_string(),
            });
            if let Some(id) = uuid {
                token["entity"] = json!({ "id": id });
            }
            blocks.push(json!({
                "id": Uuid::new_v4().to_string(),
                "type": "EntityBlock",
                "blocks": [],
                "hierarchy": { "key": "Base", "val": 0 },
                "tokens": [token]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- tokens_to_markdown ---

    #[test]
    fn tokens_to_markdown_plain() {
        let tokens = vec![
            json!({"type": "TextToken", "text": "hello", "style": {"bold": false, "italic": false}}),
        ];
        assert_eq!(tokens_to_markdown(&tokens), "hello");
    }

    #[test]
    fn tokens_to_markdown_bold() {
        let tokens = vec![
            json!({"type": "TextToken", "text": "hello", "style": {"bold": true, "italic": false}}),
        ];
        assert_eq!(tokens_to_markdown(&tokens), "**hello**");
    }

    #[test]
    fn tokens_to_markdown_italic() {
        let tokens = vec![
            json!({"type": "TextToken", "text": "hello", "style": {"bold": false, "italic": true}}),
        ];
        assert_eq!(tokens_to_markdown(&tokens), "*hello*");
    }

    #[test]
    fn tokens_to_markdown_bold_italic() {
        let tokens = vec![
            json!({"type": "TextToken", "text": "hello", "style": {"bold": true, "italic": true}}),
        ];
        assert_eq!(tokens_to_markdown(&tokens), "***hello***");
    }

    #[test]
    fn tokens_to_markdown_code() {
        let tokens = vec![json!({"type": "CodeToken", "code": "foo()"})];
        assert_eq!(tokens_to_markdown(&tokens), "`foo()`");
    }

    #[test]
    fn tokens_to_markdown_code_text_fallback() {
        let tokens = vec![json!({"type": "CodeToken", "text": "bar"})];
        assert_eq!(tokens_to_markdown(&tokens), "`bar`");
    }

    #[test]
    fn tokens_to_markdown_link() {
        let tokens =
            vec![json!({"type": "LinkToken", "text": "Click", "url": "https://example.com"})];
        assert_eq!(tokens_to_markdown(&tokens), "[Click](https://example.com)");
    }

    #[test]
    fn tokens_to_markdown_link_no_text() {
        let tokens = vec![json!({"type": "LinkToken", "url": "https://example.com"})];
        assert_eq!(
            tokens_to_markdown(&tokens),
            "[https://example.com](https://example.com)"
        );
    }

    #[test]
    fn tokens_to_markdown_unknown_type() {
        let tokens = vec![json!({"type": "FooToken", "text": "fallback"})];
        assert_eq!(tokens_to_markdown(&tokens), "fallback");
    }

    #[test]
    fn tokens_to_markdown_multiple() {
        let tokens = vec![
            json!({"type": "TextToken", "text": "Hello ", "style": {"bold": false, "italic": false}}),
            json!({"type": "TextToken", "text": "world", "style": {"bold": true, "italic": false}}),
        ];
        assert_eq!(tokens_to_markdown(&tokens), "Hello **world**");
    }

    // --- block_to_markdown ---

    #[test]
    fn block_to_markdown_text_base() {
        let block = json!({
            "type": "TextBlock",
            "tokens": [{"type": "TextToken", "text": "Hello", "style": {"bold": false, "italic": false}}],
            "hierarchy": {"key": "Base"}
        });
        assert_eq!(block_to_markdown(&block), "Hello");
    }

    #[test]
    fn block_to_markdown_headings() {
        for (level, prefix) in [("H1", "#"), ("H2", "##"), ("H3", "###"), ("H4", "####")] {
            let block = json!({
                "type": "TextBlock",
                "tokens": [{"type": "TextToken", "text": "Title", "style": {"bold": false, "italic": false}}],
                "hierarchy": {"key": level}
            });
            assert_eq!(block_to_markdown(&block), format!("{prefix} Title"));
        }
    }

    #[test]
    fn block_to_markdown_code_with_lang() {
        let block = json!({"type": "CodeBlock", "language": "rust", "code": "fn main() {}"});
        assert_eq!(block_to_markdown(&block), "```rust\nfn main() {}\n```");
    }

    #[test]
    fn block_to_markdown_code_no_lang() {
        let block = json!({"type": "CodeBlock", "language": "", "code": "x = 1"});
        assert_eq!(block_to_markdown(&block), "```\nx = 1\n```");
    }

    #[test]
    fn block_to_markdown_code_tokens_fallback() {
        let block = json!({
            "type": "CodeBlock",
            "language": "py",
            "tokens": [{"type": "TextToken", "text": "print(1)", "style": {"bold": false, "italic": false}}]
        });
        assert_eq!(block_to_markdown(&block), "```py\nprint(1)\n```");
    }

    #[test]
    fn block_to_markdown_entity() {
        let block = json!({
            "type": "EntityBlock",
            "tokens": [{"entityTitle": "My Note"}]
        });
        assert_eq!(block_to_markdown(&block), "[[My Note]]");
    }

    #[test]
    fn block_to_markdown_entity_text_fallback() {
        let block = json!({
            "type": "EntityBlock",
            "tokens": [{"text": "plain ref"}]
        });
        assert_eq!(block_to_markdown(&block), "plain ref");
    }

    #[test]
    fn block_to_markdown_entity_no_tokens() {
        let block = json!({"type": "EntityBlock"});
        assert_eq!(block_to_markdown(&block), "");
    }

    #[test]
    fn block_to_markdown_horizontal_line() {
        let block = json!({"type": "HorizontalLineBlock"});
        assert_eq!(block_to_markdown(&block), "---");
    }

    #[test]
    fn block_to_markdown_table() {
        let block = json!({
            "type": "SimpleTableBlock",
            "columns": [
                {"cells": [
                    {"tokens": [{"type": "TextToken", "text": "A", "style": {"bold": false, "italic": false}}]},
                    {"tokens": [{"type": "TextToken", "text": "1", "style": {"bold": false, "italic": false}}]}
                ]},
                {"cells": [
                    {"tokens": [{"type": "TextToken", "text": "B", "style": {"bold": false, "italic": false}}]},
                    {"tokens": [{"type": "TextToken", "text": "2", "style": {"bold": false, "italic": false}}]}
                ]}
            ]
        });
        let md = block_to_markdown(&block);
        assert_eq!(md, "| A | B |\n| --- | --- |\n| 1 | 2 |");
    }

    #[test]
    fn block_to_markdown_table_empty_columns() {
        let block = json!({"type": "SimpleTableBlock", "columns": []});
        assert_eq!(block_to_markdown(&block), "");
    }

    #[test]
    fn block_to_markdown_table_no_rows() {
        let block = json!({"type": "SimpleTableBlock", "columns": [{"cells": []}]});
        assert_eq!(block_to_markdown(&block), "");
    }

    #[test]
    fn block_to_markdown_unknown_type() {
        let block = json!({"type": "UnknownBlock"});
        assert_eq!(
            block_to_markdown(&block),
            "<!-- unsupported: UnknownBlock -->"
        );
    }

    #[test]
    fn block_to_markdown_unsupported_type() {
        let block = json!({"type": "SomeNewBlock"});
        assert_eq!(
            block_to_markdown(&block),
            "<!-- unsupported: SomeNewBlock -->"
        );
    }

    // --- blocks_to_markdown ---

    #[test]
    fn blocks_to_markdown_empty() {
        assert_eq!(blocks_to_markdown(&[]), "");
    }

    #[test]
    fn blocks_to_markdown_multiple() {
        let blocks = vec![
            json!({"type": "TextBlock", "tokens": [{"type": "TextToken", "text": "line1", "style": {"bold": false, "italic": false}}], "hierarchy": {"key": "Base"}}),
            json!({"type": "HorizontalLineBlock"}),
            json!({"type": "TextBlock", "tokens": [{"type": "TextToken", "text": "line2", "style": {"bold": false, "italic": false}}], "hierarchy": {"key": "Base"}}),
        ];
        assert_eq!(blocks_to_markdown(&blocks), "line1\n---\nline2");
    }

    // --- parse_inline_tokens ---

    #[test]
    fn parse_inline_plain() {
        let tokens = parse_inline_tokens("hello world");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["type"], "TextToken");
        assert_eq!(tokens[0]["text"], "hello world");
    }

    #[test]
    fn parse_inline_bold() {
        let tokens = parse_inline_tokens("**bold**");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["text"], "bold");
        assert_eq!(tokens[0]["style"]["bold"], true);
        assert_eq!(tokens[0]["style"]["italic"], false);
    }

    #[test]
    fn parse_inline_italic() {
        let tokens = parse_inline_tokens("*italic*");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["text"], "italic");
        assert_eq!(tokens[0]["style"]["italic"], true);
        assert_eq!(tokens[0]["style"]["bold"], false);
    }

    #[test]
    fn parse_inline_bold_italic() {
        let tokens = parse_inline_tokens("***both***");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["text"], "both");
        assert_eq!(tokens[0]["style"]["bold"], true);
        assert_eq!(tokens[0]["style"]["italic"], true);
    }

    #[test]
    fn parse_inline_code() {
        let tokens = parse_inline_tokens("`code`");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["type"], "CodeToken");
        assert_eq!(tokens[0]["code"], "code");
    }

    #[test]
    fn parse_inline_link() {
        let tokens = parse_inline_tokens("[Click](https://example.com)");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["type"], "LinkToken");
        assert_eq!(tokens[0]["text"], "Click");
        assert_eq!(tokens[0]["url"], "https://example.com");
    }

    #[test]
    fn parse_inline_mixed() {
        let tokens = parse_inline_tokens("Hello **bold** and *italic*");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0]["text"], "Hello ");
        assert_eq!(tokens[1]["text"], "bold");
        assert_eq!(tokens[1]["style"]["bold"], true);
        assert_eq!(tokens[2]["text"], " and ");
        assert_eq!(tokens[3]["text"], "italic");
        assert_eq!(tokens[3]["style"]["italic"], true);
    }

    #[test]
    fn parse_inline_empty() {
        let tokens = parse_inline_tokens("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0]["text"], "");
    }

    // --- markdown_to_blocks ---

    #[test]
    fn md_to_blocks_headings() {
        let blocks = markdown_to_blocks("# H1\n## H2\n### H3\n#### H4");
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0]["hierarchy"]["key"], "H1");
        assert_eq!(blocks[1]["hierarchy"]["key"], "H2");
        assert_eq!(blocks[2]["hierarchy"]["key"], "H3");
        assert_eq!(blocks[3]["hierarchy"]["key"], "H4");
    }

    #[test]
    fn md_to_blocks_code_fence() {
        let blocks = markdown_to_blocks("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "CodeBlock");
        assert_eq!(blocks[0]["language"], "rust");
        assert_eq!(blocks[0]["code"], "fn main() {}");
    }

    #[test]
    fn md_to_blocks_code_fence_no_lang() {
        let blocks = markdown_to_blocks("```\nsome code\n```");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "CodeBlock");
        assert_eq!(blocks[0]["language"], "");
    }

    #[test]
    fn md_to_blocks_horizontal_rule() {
        for hr in ["---", "***", "___"] {
            let blocks = markdown_to_blocks(hr);
            assert_eq!(blocks.len(), 1, "failed for {hr}");
            assert_eq!(blocks[0]["type"], "HorizontalLineBlock");
        }
    }

    #[test]
    fn md_to_blocks_plain_text() {
        let blocks = markdown_to_blocks("Hello world");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "TextBlock");
        assert_eq!(blocks[0]["hierarchy"]["key"], "Base");
    }

    #[test]
    fn md_to_blocks_empty() {
        let blocks = markdown_to_blocks("");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "TextBlock");
    }

    // --- Roundtrip ---

    #[test]
    fn roundtrip_plain_text() {
        let md = "Hello world";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_heading() {
        let md = "## My Heading";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_code_block() {
        let md = "```rust\nlet x = 1;\n```";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_horizontal_rule() {
        let blocks = markdown_to_blocks("---");
        let result = blocks_to_markdown(&blocks);
        assert_eq!(result, "---");
    }

    #[test]
    fn roundtrip_bold_italic() {
        let md = "Hello **bold** and *italic* text";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_inline_code() {
        let md = "Use `foo()` here";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_link() {
        let md = "Visit [Example](https://example.com) now";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_multi_block() {
        let md = "# Title\nSome text\n---\n```py\nprint(1)\n```";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    // --- Edge cases ---

    #[test]
    fn code_block_with_special_chars() {
        let md = "```js\nconst x = '<div class=\"foo\">';\n```";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn code_block_with_backticks_inside() {
        let md = "```\nuse `backtick` inside\n```";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn entity_block_uuid_preserved() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let block = json!({
            "type": "EntityBlock",
            "tokens": [{
                "entityTitle": "Test Entity",
                "entity": { "id": uuid }
            }]
        });
        let md = block_to_markdown(&block);
        assert_eq!(md, format!("[[Test Entity|{uuid}]]"));
    }

    #[test]
    fn entity_block_no_uuid() {
        let block = json!({
            "type": "EntityBlock",
            "tokens": [{"entityTitle": "No UUID Entity"}]
        });
        assert_eq!(block_to_markdown(&block), "[[No UUID Entity]]");
    }

    #[test]
    fn roundtrip_wikilink_with_uuid() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let md = format!("[[My Entity|{uuid}]]");
        let blocks = markdown_to_blocks(&md);
        assert_eq!(blocks[0]["type"], "EntityBlock");
        let result = blocks_to_markdown(&blocks);
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_wikilink_no_uuid() {
        let md = "[[Simple Link]]";
        let blocks = markdown_to_blocks(md);
        assert_eq!(blocks[0]["type"], "EntityBlock");
        let result = blocks_to_markdown(&blocks);
        assert_eq!(result, md);
    }

    #[test]
    fn nested_inline_bold_and_code() {
        let tokens = parse_inline_tokens("**bold** then `code` then *italic*");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0]["text"], "bold");
        assert_eq!(tokens[0]["style"]["bold"], true);
        assert_eq!(tokens[1]["text"], " then ");
        assert_eq!(tokens[2]["type"], "CodeToken");
        assert_eq!(tokens[2]["code"], "code");
        assert_eq!(tokens[3]["text"], " then ");
        assert_eq!(tokens[4]["text"], "italic");
        assert_eq!(tokens[4]["style"]["italic"], true);
    }

    #[test]
    fn multiline_text_blocks() {
        let md = "Line one\nLine two\nLine three";
        let blocks = markdown_to_blocks(md);
        assert_eq!(blocks.len(), 3);
        for b in &blocks {
            assert_eq!(b["type"], "TextBlock");
        }
    }

    #[test]
    fn heading_with_inline_formatting() {
        let md = "## Hello **world**";
        let blocks = markdown_to_blocks(md);
        assert_eq!(blocks[0]["hierarchy"]["key"], "H2");
        let result = blocks_to_markdown(&blocks);
        assert_eq!(result, md);
    }

    #[test]
    fn tokens_to_markdown_empty_text_token() {
        let tokens = vec![
            json!({"type": "TextToken", "text": "", "style": {"bold": false, "italic": false}}),
        ];
        assert_eq!(tokens_to_markdown(&tokens), "");
    }

    #[test]
    fn text_block_no_tokens() {
        let block = json!({"type": "TextBlock", "hierarchy": {"key": "Base"}});
        assert_eq!(block_to_markdown(&block), "");
    }

    // --- New block type tests ---

    #[test]
    fn roundtrip_checklist_checked() {
        let md = "- [x] Done task";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_checklist_unchecked() {
        let md = "- [ ] Pending task";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_bullet_list() {
        let md = "- Item one\n- Item two\n- Item three";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_numbered_list() {
        let md = "1. First\n2. Second\n3. Third";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_quote() {
        let md = "> This is a quote";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn roundtrip_callout() {
        let md = "> [!NOTE] Important note here";
        let result = blocks_to_markdown(&markdown_to_blocks(md));
        assert_eq!(result, md);
    }

    #[test]
    fn block_to_markdown_toggle() {
        let block = json!({
            "type": "ToggleBlock",
            "tokens": [{"type": "TextToken", "text": "Title", "style": {"bold": false, "italic": false}}],
            "blocks": [
                {"type": "TextBlock", "tokens": [{"type": "TextToken", "text": "Content", "style": {"bold": false, "italic": false}}], "hierarchy": {"key": "Base"}}
            ]
        });
        assert_eq!(
            block_to_markdown(&block),
            "<details><summary>Title</summary>Content</details>"
        );
    }

    #[test]
    fn block_to_markdown_image_with_url() {
        let block = json!({
            "type": "ImageBlock",
            "url": "https://example.com/img.png",
            "alt": "My Image"
        });
        assert_eq!(
            block_to_markdown(&block),
            "![My Image](https://example.com/img.png)"
        );
    }

    #[test]
    fn block_to_markdown_image_no_url() {
        let block = json!({
            "type": "ImageBlock",
            "filename": "photo.jpg"
        });
        assert_eq!(block_to_markdown(&block), "[Image: photo.jpg]");
    }

    #[test]
    fn md_to_blocks_checklist() {
        let blocks = markdown_to_blocks("- [x] done\n- [ ] pending");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "ChecklistBlock");
        assert_eq!(blocks[0]["checked"], true);
        assert_eq!(blocks[1]["type"], "ChecklistBlock");
        assert_eq!(blocks[1]["checked"], false);
    }

    #[test]
    fn md_to_blocks_bullet_list() {
        let blocks = markdown_to_blocks("- item one\n- item two");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "BulletListBlock");
        assert_eq!(blocks[1]["type"], "BulletListBlock");
    }

    #[test]
    fn md_to_blocks_numbered_list() {
        let blocks = markdown_to_blocks("1. first\n2. second");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "NumberedListBlock");
        assert_eq!(blocks[0]["number"], 1);
        assert_eq!(blocks[1]["type"], "NumberedListBlock");
        assert_eq!(blocks[1]["number"], 2);
    }

    #[test]
    fn md_to_blocks_quote() {
        let blocks = markdown_to_blocks("> quoted text");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "QuoteBlock");
    }

    #[test]
    fn md_to_blocks_callout() {
        let blocks = markdown_to_blocks("> [!WARNING] Be careful");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "CalloutBlock");
        assert_eq!(blocks[0]["calloutType"], "WARNING");
    }

    #[test]
    fn checklist_with_inline_formatting() {
        let md = "- [x] **Important** task";
        let blocks = markdown_to_blocks(md);
        assert_eq!(blocks[0]["type"], "ChecklistBlock");
        let result = blocks_to_markdown(&blocks);
        assert_eq!(result, md);
    }

    #[test]
    fn bullet_not_confused_with_hr() {
        let blocks = markdown_to_blocks("- single item");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "BulletListBlock");
    }

    #[test]
    fn text_block_missing_hierarchy() {
        let block = json!({
            "type": "TextBlock",
            "tokens": [{"type": "TextToken", "text": "hello", "style": {"bold": false, "italic": false}}]
        });
        assert_eq!(block_to_markdown(&block), "hello");
    }
}
