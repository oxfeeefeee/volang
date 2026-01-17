//! Detra Renderable - shared types between engine and renderer.

use std::collections::HashMap;
use std::fmt;

#[derive(Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Struct(String, HashMap<String, Value>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Struct(_, _) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
            Value::Struct(_, _) => "struct",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn get_field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Map(m) => m.get(name),
            Value::Struct(_, fields) => fields.get(name),
            _ => None,
        }
    }

    pub fn get_index(&self, index: &Value) -> Option<&Value> {
        match (self, index) {
            (Value::Array(a), Value::Int(i)) => {
                let idx = *i as usize;
                a.get(idx)
            }
            (Value::Map(m), Value::String(k)) => m.get(k),
            _ => None,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Array(a) => write!(f, "{:?}", a),
            Value::Map(m) => write!(f, "{:?}", m),
            Value::Struct(name, fields) => write!(f, "{}({:?})", name, fields),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

#[derive(Debug, Clone)]
pub struct ActionCall {
    pub name: String,
    pub args: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct RuntimeNode {
    pub kind: String,
    pub key: Option<Value>,
    pub props: HashMap<String, Value>,
    pub events: HashMap<String, ActionCall>,
    pub children: Vec<RuntimeNode>,
}

impl RuntimeNode {
    pub fn empty() -> Self {
        RuntimeNode {
            kind: "Empty".to_string(),
            key: None,
            props: HashMap::new(),
            events: HashMap::new(),
            children: Vec::new(),
        }
    }
}

// ============================================================================
// Layout Output Types - Used by Renderer (no layout calculation needed)
// ============================================================================

/// Absolute rectangle (calculated by layout engine)
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width &&
        py >= self.y && py < self.y + self.height
    }
}

/// RGBA color
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
}

/// Border style
#[derive(Debug, Clone, Default)]
pub struct Border {
    pub width: f32,
    pub color: Color,
    pub radius: f32,
}

/// Text style
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
    pub monospace: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: 14.0,
            color: Color::WHITE,
            bold: false,
            italic: false,
            monospace: false,
        }
    }
}

/// Render node types - each variant contains all info needed to render
#[derive(Debug, Clone)]
pub enum RenderKind {
    /// Container - background and border only, children rendered separately
    Container {
        background: Option<Color>,
        border: Option<Border>,
    },
    
    /// Text label
    Text {
        content: String,
        style: TextStyle,
    },
    
    /// Clickable button
    Button {
        text: String,
        style: TextStyle,
        background: Color,
        border: Option<Border>,
        active: bool,
    },
    
    /// Text input
    Input {
        value: String,
        placeholder: String,
        multiline: bool,
        style: TextStyle,
        background: Color,
    },
    
    /// Divider line
    Divider {
        color: Color,
        vertical: bool,
    },
    
    /// Spacer (invisible)
    Spacer,
    
    /// Image
    Image {
        src: String,
    },
}

/// Render node - fully laid out, ready to render
#[derive(Debug, Clone)]
pub struct RenderNode {
    /// Unique ID for interaction tracking
    pub id: usize,
    
    /// Absolute position and size (calculated by layout engine)
    pub rect: Rect,
    
    /// What to render
    pub kind: RenderKind,
    
    /// Click handler (action name + args)
    pub on_click: Option<ActionCall>,
    
    /// Change handler for inputs
    pub on_change: Option<ActionCall>,
    
    /// Is this node focusable?
    pub focusable: bool,
    
    /// Child nodes (already have absolute positions)
    pub children: Vec<RenderNode>,
    
    /// Clip children to this rect (for scrolling)
    pub clip: Option<Rect>,
    
    /// Is visible?
    pub visible: bool,
}

impl RenderNode {
    pub fn new(id: usize, rect: Rect, kind: RenderKind) -> Self {
        Self {
            id,
            rect,
            kind,
            on_click: None,
            on_change: None,
            focusable: false,
            children: Vec::new(),
            clip: None,
            visible: true,
        }
    }
}

/// Complete render tree with metadata
#[derive(Debug, Clone)]
pub struct RenderTree {
    pub root: RenderNode,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub version: u64,
}

impl RenderTree {
    /// Find the deepest interactive node at position (x, y).
    /// Returns the node with on_click handler that contains the point.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&RenderNode> {
        hit_test_node(&self.root, x, y)
    }
}

fn hit_test_node(node: &RenderNode, x: f32, y: f32) -> Option<&RenderNode> {
    // Skip invisible nodes
    if !node.visible {
        return None;
    }
    
    // Check if point is inside this node's rect
    if !node.rect.contains(x, y) {
        return None;
    }
    
    // Check children first (depth-first, reverse order for z-index)
    for child in node.children.iter().rev() {
        if let Some(hit) = hit_test_node(child, x, y) {
            return Some(hit);
        }
    }
    
    // If this node has click handler, return it
    if node.on_click.is_some() {
        return Some(node);
    }
    
    None
}
