// common/src/ui/css_engine.rs

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::syscall::syscall_log;

// Temporary log function for V-Nodes
fn log(msg: &str) {
    let _ = syscall_log(msg);
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

    // Basic CSS parsing for `selector { key: value; ... }` blocks.
    pub fn parse_css(&self, css: &str) -> Vec<CssRule> {
        log("CssEngine: Parsing CSS.");

        css.split('}')
            .filter_map(|block| {
                let (selector, declarations) = block.split_once('{')?;
                let selector = selector.trim();
                if selector.is_empty() {
                    return None;
                }

                let properties = declarations
                    .split(';')
                    .filter_map(|declaration| {
                        let (name, value) = declaration.split_once(':')?;
                        let name = name.trim();
                        let value = value.trim();
                        if name.is_empty() || value.is_empty() {
                            return None;
                        }
                        Some(CssProperty {
                            name: String::from(name),
                            value: String::from(value),
                        })
                    })
                    .collect::<Vec<_>>();

                if properties.is_empty() {
                    return None;
                }

                Some(CssRule {
                    selector: String::from(selector),
                    properties,
                })
            })
            .collect()
    }

    // Applies CSS rules to a DOM node and its children (conceptual)
    pub fn apply_styles(&self, dom: &crate::ui::html_parser::DomNode, rules: &[CssRule]) -> BTreeMap<String, String> {
        log("CssEngine: Applying styles.");
        let mut styles = BTreeMap::new();

        for rule in rules {
            if selector_matches(dom, &rule.selector) {
                for property in &rule.properties {
                    styles.insert(property.name.clone(), property.value.clone());
                }
            }
        }

        styles
    }
}

fn selector_matches(dom: &crate::ui::html_parser::DomNode, selector: &str) -> bool {
    let selector = selector.trim();
    if selector.is_empty() {
        return false;
    }

    let tag = match dom {
        crate::ui::html_parser::DomNode::Element { tag_name, .. } => tag_name.as_str(),
        crate::ui::html_parser::DomNode::Text(_) => return false,
    };

    selector
        .split(',')
        .map(str::trim)
        .any(|single| single == "*" || single.eq_ignore_ascii_case(tag))
}
