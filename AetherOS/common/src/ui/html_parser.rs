// common/src/ui/html_parser.rs

#![no_std]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::syscall::{syscall3, SYS_LOG, SUCCESS};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
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
        log(&alloc::format!("HtmlParser: Parsing HTML (stub): {}", html));
        // In a real implementation, this would build a proper DOM tree.
        DomNode::Element {
            tag_name: String::from("html"),
            attributes: Vec::new(),
            children: vec![
                DomNode::Element { 
                    tag_name: String::from("body"), 
                    attributes: Vec::new(), 
                    children: vec![DomNode::Text(String::from("Hello from WebView!"))] 
                }
            ],
        }
    }
}