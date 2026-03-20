// common/src/ui/html_parser.rs


extern crate alloc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::syscall::{syscall3, SYS_LOG, SUCCESS};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    let res = syscall3(
        SYS_LOG,
        msg.as_ptr() as u64,
        msg.len() as u64,
        0 // arg3 is unused for SYS_LOG
    );
    if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
}

/// Represents a simplified HTML DOM node.
#[derive(Debug, PartialEq)]
pub enum DomNode {
    Element { tag_name: String, attributes: Vec<(String, String)>, children: Vec<DomNode> },
    Text(String),
}

pub struct HtmlParser;

impl HtmlParser {
    pub fn new() -> Self { HtmlParser { } }

    // Very basic conceptual parsing
    pub fn parse_html(&self, html: &str) -> DomNode {
        log(&alloc::format!("HtmlParser: Parsing HTML: {}", html));

        let body_text = extract_body_text(html)
            .or_else(|| extract_text_content(html))
            .unwrap_or_else(|| String::from("Hello from WebView!"));

        // This remains intentionally minimal, but now preserves incoming text content.
        DomNode::Element {
            tag_name: String::from("html"),
            attributes: Vec::new(),
            children: vec![
                DomNode::Element {
                    tag_name: String::from("body"),
                    attributes: Vec::new(),
                    children: vec![DomNode::Text(body_text)]
                }
            ],
        }
    }
}

fn extract_body_text(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let body_start = lower.find("<body")?;
    let content_start = lower[body_start..].find('>')? + body_start + 1;
    let body_end = lower[content_start..].find("</body>")? + content_start;

    let raw = &html[content_start..body_end];
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(String::from(trimmed))
    }
}

fn extract_text_content(html: &str) -> Option<String> {
    let mut in_tag = false;
    let mut output = String::new();

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(c),
            _ => {}
        }
    }

    let trimmed = output.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(String::from(trimmed))
    }
}
