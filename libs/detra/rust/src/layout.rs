//! Layout engine - converts RuntimeNode to RenderTree using taffy flexbox.

use std::collections::HashMap;
use taffy::prelude::*;
use detra_renderable::{
    RuntimeNode, RenderNode, RenderTree, RenderKind,
    Rect as DetraRect, Color, Border, TextStyle, ActionCall,
};

/// Compute layout for a RuntimeNode tree, producing a RenderTree with absolute positions.
pub fn layout(tree: &RuntimeNode, viewport_width: f32, viewport_height: f32) -> RenderTree {
    let mut taffy = TaffyTree::new();
    let mut node_map: HashMap<NodeId, NodeInfo> = HashMap::new();
    let mut id_counter = 0usize;
    
    // Build taffy tree from RuntimeNode
    let root_id = build_taffy_tree(&mut taffy, tree, &mut node_map, &mut id_counter);
    
    // Compute layout
    taffy.compute_layout(
        root_id,
        Size {
            width: AvailableSpace::Definite(viewport_width),
            height: AvailableSpace::Definite(viewport_height),
        },
    ).unwrap();
    
    // Convert to RenderTree
    let root = build_render_tree(&taffy, root_id, &node_map, 0.0, 0.0);
    
    RenderTree {
        root,
        viewport_width,
        viewport_height,
        version: 0,
    }
}

struct NodeInfo {
    id: usize,
    runtime_node: RuntimeNode,
}

fn build_taffy_tree(
    taffy: &mut TaffyTree,
    node: &RuntimeNode,
    node_map: &mut HashMap<NodeId, NodeInfo>,
    id_counter: &mut usize,
) -> NodeId {
    // Build children first
    let child_ids: Vec<NodeId> = node.children
        .iter()
        .map(|child| build_taffy_tree(taffy, child, node_map, id_counter))
        .collect();
    
    // Create style from props
    let style = create_taffy_style(node);
    
    // Create taffy node
    let taffy_id = taffy.new_with_children(style, &child_ids).unwrap();
    
    // Store mapping
    let id = *id_counter;
    *id_counter += 1;
    node_map.insert(taffy_id, NodeInfo {
        id,
        runtime_node: node.clone(),
    });
    
    taffy_id
}

fn create_taffy_style(node: &RuntimeNode) -> Style {
    let mut style = Style::default();
    
    // Direction based on node kind
    match node.kind.as_str() {
        "Column" => {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Column;
        }
        "Row" => {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Row;
        }
        _ => {
            style.display = Display::Flex;
        }
    }
    
    // Width
    if let Some(w) = get_float_prop(&node.props, "width") {
        style.size.width = Dimension::Length(w);
    } else if get_bool_prop(&node.props, "fill") {
        style.size.width = Dimension::Percent(1.0);
    }
    
    // Height
    if let Some(h) = get_float_prop(&node.props, "height") {
        style.size.height = Dimension::Length(h);
    } else if get_bool_prop(&node.props, "fill") {
        style.size.height = Dimension::Percent(1.0);
    }
    
    // Flex
    if let Some(flex) = get_float_prop(&node.props, "flex") {
        style.flex_grow = flex;
        style.flex_shrink = 1.0;
        style.flex_basis = Dimension::Length(0.0);
    }
    
    // Padding
    if let Some(p) = get_float_prop(&node.props, "padding") {
        style.padding = Rect {
            left: LengthPercentage::Length(p),
            right: LengthPercentage::Length(p),
            top: LengthPercentage::Length(p),
            bottom: LengthPercentage::Length(p),
        };
    }
    
    // Gap/Spacing
    if let Some(s) = get_float_prop(&node.props, "spacing") {
        style.gap = Size {
            width: LengthPercentage::Length(s),
            height: LengthPercentage::Length(s),
        };
    }
    
    // Min/Max constraints
    if let Some(min_w) = get_float_prop(&node.props, "minWidth") {
        style.min_size.width = Dimension::Length(min_w);
    }
    if let Some(max_w) = get_float_prop(&node.props, "maxWidth") {
        style.max_size.width = Dimension::Length(max_w);
    }
    if let Some(min_h) = get_float_prop(&node.props, "minHeight") {
        style.min_size.height = Dimension::Length(min_h);
    }
    if let Some(max_h) = get_float_prop(&node.props, "maxHeight") {
        style.max_size.height = Dimension::Length(max_h);
    }
    
    // Intrinsic sizes for leaf nodes
    match node.kind.as_str() {
        "Text" => {
            let text = get_string_prop(&node.props, "text").unwrap_or_default();
            let size = get_float_prop(&node.props, "size").unwrap_or(14.0);
            // Estimate text size
            let char_width = size * 0.6;
            let estimated_width = text.len() as f32 * char_width;
            if style.size.width == Dimension::Auto {
                style.size.width = Dimension::Length(estimated_width.max(10.0));
            }
            if style.size.height == Dimension::Auto {
                style.size.height = Dimension::Length(size * 1.4);
            }
        }
        "Button" => {
            let text = get_string_prop(&node.props, "text").unwrap_or_default();
            let size = get_float_prop(&node.props, "size").unwrap_or(14.0);
            let char_width = size * 0.6;
            let estimated_width = text.len() as f32 * char_width + 16.0; // padding
            if style.size.width == Dimension::Auto {
                style.size.width = Dimension::Length(estimated_width.max(40.0));
            }
            if style.size.height == Dimension::Auto {
                style.size.height = Dimension::Length(size * 1.4 + 8.0);
            }
        }
        "Input" => {
            if style.size.height == Dimension::Auto {
                let multiline = get_bool_prop(&node.props, "multiline");
                style.size.height = Dimension::Length(if multiline { 100.0 } else { 28.0 });
            }
            if style.size.width == Dimension::Auto {
                style.flex_grow = 1.0;
            }
        }
        "Divider" => {
            let vertical = get_bool_prop(&node.props, "vertical");
            if vertical {
                style.size.width = Dimension::Length(1.0);
            } else {
                style.size.height = Dimension::Length(1.0);
            }
        }
        "Spacer" => {
            if let Some(size) = get_float_prop(&node.props, "size") {
                style.size.width = Dimension::Length(size);
                style.size.height = Dimension::Length(size);
            } else {
                style.flex_grow = 1.0;
            }
        }
        _ => {}
    }
    
    style
}

fn build_render_tree(
    taffy: &TaffyTree,
    taffy_id: NodeId,
    node_map: &HashMap<NodeId, NodeInfo>,
    parent_x: f32,
    parent_y: f32,
) -> RenderNode {
    let layout = taffy.layout(taffy_id).unwrap();
    let info = node_map.get(&taffy_id).unwrap();
    let runtime = &info.runtime_node;
    
    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    let width = layout.size.width;
    let height = layout.size.height;
    
    let rect = DetraRect::new(x, y, width, height);
    
    // Create RenderKind based on node type
    let kind = create_render_kind(runtime);
    
    // Build children
    let children: Vec<RenderNode> = taffy.children(taffy_id)
        .unwrap()
        .iter()
        .map(|&child_id| build_render_tree(taffy, child_id, node_map, x, y))
        .collect();
    
    // Extract events - already detra_renderable::ActionCall, no conversion needed
    let on_click = runtime.events.get("onClick").cloned();
    let on_change = runtime.events.get("onChange").cloned();
    
    let focusable = on_click.is_some() || matches!(kind, RenderKind::Button { .. } | RenderKind::Input { .. });
    
    RenderNode {
        id: info.id,
        rect,
        kind,
        on_click,
        on_change,
        focusable,
        children,
        clip: None,
        visible: true,
    }
}

fn create_render_kind(node: &RuntimeNode) -> RenderKind {
    match node.kind.as_str() {
        "Column" | "Row" => {
            let bg = get_style_background(&node.props);
            RenderKind::Container {
                background: bg,
                border: None,
            }
        }
        "Text" => {
            let content = get_string_prop(&node.props, "text").unwrap_or_default();
            let style = create_text_style(&node.props);
            RenderKind::Text { content, style }
        }
        "Button" => {
            let text = get_string_prop(&node.props, "text").unwrap_or_default();
            let style = create_text_style(&node.props);
            let active = get_bool_prop(&node.props, "active");
            let bg = get_style_background(&node.props).unwrap_or(Color::rgb(60, 60, 60));
            RenderKind::Button {
                text,
                style,
                background: bg,
                border: None,
                active,
            }
        }
        "Input" => {
            let value = get_string_prop(&node.props, "value").unwrap_or_default();
            let placeholder = get_string_prop(&node.props, "placeholder").unwrap_or_default();
            let multiline = get_bool_prop(&node.props, "multiline");
            let style = create_text_style(&node.props);
            RenderKind::Input {
                value,
                placeholder,
                multiline,
                style,
                background: Color::rgb(30, 30, 30),
            }
        }
        "Divider" => {
            let vertical = get_bool_prop(&node.props, "vertical");
            RenderKind::Divider {
                color: Color::rgb(60, 60, 60),
                vertical,
            }
        }
        "Spacer" => RenderKind::Spacer,
        "Image" => {
            let src = get_string_prop(&node.props, "src").unwrap_or_default();
            RenderKind::Image { src }
        }
        _ => RenderKind::Container {
            background: None,
            border: None,
        }
    }
}

fn create_text_style(props: &HashMap<String, detra_renderable::Value>) -> TextStyle {
    let size = get_float_prop(props, "size").unwrap_or(14.0);
    let bold = get_bool_prop(props, "bold");
    let color = get_color_prop(props, "color").unwrap_or(Color::WHITE);
    let monospace = get_string_prop(props, "font").map(|f| f == "monospace").unwrap_or(false);
    
    TextStyle {
        size,
        color,
        bold,
        italic: false,
        monospace,
    }
}

fn get_style_background(props: &HashMap<String, detra_renderable::Value>) -> Option<Color> {
    if let Some(style) = get_string_prop(props, "style") {
        match style.as_str() {
            "sidebar" => Some(Color::rgb(37, 37, 38)),
            "panel" => Some(Color::rgb(37, 37, 38)),
            "menubar" => Some(Color::rgb(50, 50, 50)),
            "tabbar" => Some(Color::rgb(45, 45, 45)),
            "statusbar" => Some(Color::rgb(0, 122, 204)),
            "editor" => Some(Color::rgb(30, 30, 30)),
            _ => None,
        }
    } else {
        None
    }
}

// Helper functions
fn get_float_prop(props: &HashMap<String, detra_renderable::Value>, key: &str) -> Option<f32> {
    match props.get(key) {
        Some(detra_renderable::Value::Int(n)) => Some(*n as f32),
        Some(detra_renderable::Value::Float(f)) => Some(*f as f32),
        _ => None,
    }
}

fn get_bool_prop(props: &HashMap<String, detra_renderable::Value>, key: &str) -> bool {
    match props.get(key) {
        Some(detra_renderable::Value::Bool(b)) => *b,
        _ => false,
    }
}

fn get_string_prop(props: &HashMap<String, detra_renderable::Value>, key: &str) -> Option<String> {
    match props.get(key) {
        Some(detra_renderable::Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn get_color_prop(props: &HashMap<String, detra_renderable::Value>, key: &str) -> Option<Color> {
    let s = get_string_prop(props, key)?;
    parse_color(&s)
}

fn parse_color(s: &str) -> Option<Color> {
    match s {
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        "gray" | "grey" => Some(Color::rgb(128, 128, 128)),
        "red" => Some(Color::rgb(244, 67, 54)),
        "green" => Some(Color::rgb(76, 175, 80)),
        "blue" => Some(Color::rgb(33, 150, 243)),
        "yellow" => Some(Color::rgb(255, 235, 59)),
        "orange" => Some(Color::rgb(255, 152, 0)),
        "purple" => Some(Color::rgb(156, 39, 176)),
        "cyan" => Some(Color::rgb(0, 188, 212)),
        _ if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        _ => None,
    }
}
