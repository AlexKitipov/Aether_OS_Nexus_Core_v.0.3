// common/src/ui/layout.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::syscall::{syscall3, SYS_LOG, SUCCESS};
use crate::ui::html_parser::DomNode;

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

/// Represents the computed layout for a DOM node.
#[derive(Debug, PartialEq)]
pub struct LayoutBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub content_width: u32,
    pub content_height: u32,
    pub children: Vec<LayoutBox>,
    pub debug_name: String, // For debugging purposes
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn new() -> Self { LayoutEngine { } }

    // Very basic conceptual layout calculation
    pub fn layout(&self, dom: &DomNode, _computed_styles: &BTreeMap<String, String>, viewport_width: u32, viewport_height: u32) -> LayoutBox {
        log("LayoutEngine: Performing layout (stub).");

        let root_box = LayoutBox {
            x: 0,
            y: 0,
            width: viewport_width,
            height: viewport_height,
            content_width: viewport_width,
            content_height: viewport_height,
            children: Vec::new(),
            debug_name: String::from("root"),
        };

        match dom {
            DomNode::Element { tag_name, children, .. } => {
                let mut children_layouts = Vec::new();
                let mut current_y = 0;
                for child in children {
                    // Simple stacking layout
                    let child_layout = self.layout(child, _computed_styles, viewport_width, viewport_height);
                    children_layouts.push(LayoutBox { 
                        x: 0, y: current_y, 
                        width: child_layout.width, 
                        height: child_layout.height, 
                        content_width: child_layout.content_width, 
                        content_height: child_layout.content_height, 
                        children: child_layout.children, 
                        debug_name: alloc::format!("{}-child", tag_name) 
                    });
                    current_y += child_layout.height;
                }
                LayoutBox {
                    x: root_box.x,
                    y: root_box.y,
                    width: root_box.width,
                    height: root_box.height,
                    content_width: root_box.content_width,
                    content_height: current_y, // Sum of children height for conceptual content height
                    children: children_layouts,
                    debug_name: tag_name.clone(),
                }
            },
            DomNode::Text(text) => {
                // Simple text layout: assume a fixed line height and character width
                let char_width = 8; // Pixels per character
                let line_height = 20; // Pixels per line
                let width = (text.len() * char_width).min(viewport_width as usize) as u32;
                let height = line_height;
                LayoutBox {
                    x: 0,
                    y: 0,
                    width,
                    height,
                    content_width: width,
                    content_height: height,
                    children: Vec::new(),
                    debug_name: String::from("text"),
                }
            },
        }
    }
}