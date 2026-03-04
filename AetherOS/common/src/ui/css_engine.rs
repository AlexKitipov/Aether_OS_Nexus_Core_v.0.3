// common/src/ui/css_engine.rs

#![no_std]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

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

/// Represents a simplified CSS property and value.
#[derive(Debug, PartialEq)]
pub struct CssProperty {
    pub name: String,
    pub value: String,
}

/// Represents a simplified CSS rule with a selector and properties.
#[derive(Debug, PartialEq)]
pub struct CssRule {
    pub selector: String,
    pub properties: Vec<CssProperty>,
}

pub struct CssEngine;

impl CssEngine {
    pub fn new() -> Self { CssEngine { } }

    // Very basic conceptual parsing of CSS
    pub fn parse_css(&self, css: &str) -> Vec<CssRule> {
        log(&alloc::format!("CssEngine: Parsing CSS (stub): {}", css));
        // In a real implementation, this would parse CSS rules.
        vec![
            CssRule {
                selector: String::from("body"),
                properties: vec![
                    CssProperty { name: String::from("background-color"), value: String::from("white") },
                    CssProperty { name: String::from("color"), value: String::from("black") },
                ],
            },
        ]
    }

    // Applies CSS rules to a DOM node and its children (conceptual)
    pub fn apply_styles(&self, _dom: &crate::ui::html_parser::DomNode, _rules: &[CssRule]) -> BTreeMap<String, String> {
        log("CssEngine: Applying styles (stub).");
        // This would compute the final styles for each element.
        let mut styles = BTreeMap::new();
        styles.insert(String::from("color"), String::from("black"));
        styles.insert(String::from("font-size"), String::from("16px"));
        styles
    }
}
