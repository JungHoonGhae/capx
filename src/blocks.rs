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
        _ => String::new(),
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
        assert_eq!(block_to_markdown(&block), "");
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
}
