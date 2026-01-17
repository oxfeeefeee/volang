//! Detra Renderer TUI - Terminal-based rendering engine for Detra UI trees.
//!
//! Renders Detra UI to the terminal using ratatui/crossterm.
//! Supports VSCode-like IDE layouts with:
//! - Flex layout (width/height/flex/fill)
//! - Box drawing for panels and borders
//! - Text styling (bold, color)
//! - Interactive components (buttons, inputs)

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, stdout, Stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use linkme::distributed_slice;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use vo_runtime::ffi::{ExternCallContext, ExternEntryWithContext, ExternResult, EXTERN_TABLE_WITH_CONTEXT};
use vo_runtime::gc::GcRef;
use vo_runtime::objects::string;

use detra_renderable::{ActionCall, RuntimeNode, Value};

type DetraGetCurrentTree = unsafe extern "C" fn() -> *const RuntimeNode;

fn get_detra_get_current_tree() -> Option<DetraGetCurrentTree> {
    #[cfg(unix)]
    unsafe {
        let handle = libc::dlsym(libc::RTLD_DEFAULT, b"detra_get_current_tree\0".as_ptr() as *const _);
        if handle.is_null() {
            None
        } else {
            Some(std::mem::transmute::<*mut libc::c_void, DetraGetCurrentTree>(handle))
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn load_current_tree() -> Option<RuntimeNode> {
    let get_tree = get_detra_get_current_tree()?;
    unsafe {
        let root = get_tree();
        if root.is_null() {
            return None;
        }
        Some((*root).clone())
    }
}

thread_local! {
    static CURRENT_TREE: RefCell<Option<RuntimeNode>> = const { RefCell::new(None) };
    static PENDING_ACTIONS: RefCell<Vec<ActionCall>> = const { RefCell::new(Vec::new()) };
    static CURRENT_ACTION_NAME: RefCell<String> = const { RefCell::new(String::new()) };
    static CURRENT_ACTION_ARGS: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
    static FOCUS_INDEX: RefCell<usize> = const { RefCell::new(0) };
    static FOCUSABLE_ACTIONS: RefCell<Vec<ActionCall>> = const { RefCell::new(Vec::new()) };
}

struct TuiApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    on_action_closure: GcRef,
    vm: *mut std::ffi::c_void,
    fiber: *mut std::ffi::c_void,
    call_closure_fn: vo_runtime::ffi::ClosureCallFn,
}

impl TuiApp {
    fn new(
        on_action_closure: GcRef,
        vm: *mut std::ffi::c_void,
        fiber: *mut std::ffi::c_void,
        call_closure_fn: vo_runtime::ffi::ClosureCallFn,
    ) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            on_action_closure,
            vm,
            fiber,
            call_closure_fn,
        })
    }

    fn run(&mut self, max_frames: Option<usize>) -> io::Result<()> {
        let mut frame_count = 0;
        loop {
            // Render
            self.terminal.draw(|frame| {
                let tree = CURRENT_TREE.with(|cell| cell.borrow().clone());
                if let Some(tree) = tree {
                    FOCUSABLE_ACTIONS.with(|cell| cell.borrow_mut().clear());
                    render_node(frame, frame.area(), &tree);
                } else {
                    frame.render_widget(
                        Paragraph::new("Loading..."),
                        frame.area(),
                    );
                }
            })?;

            frame_count += 1;
            if let Some(max) = max_frames {
                if frame_count >= max {
                    break;
                }
            }

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Tab => {
                                // Just cycle focus visually, don't dispatch any action
                                FOCUS_INDEX.with(|cell| {
                                    let count = FOCUSABLE_ACTIONS.with(|a| a.borrow().len());
                                    if count > 0 {
                                        *cell.borrow_mut() = (*cell.borrow() + 1) % count;
                                    }
                                });
                            }
                            KeyCode::Enter => {
                                // TODO: Fix closure call - currently causes crash
                                // For now, just store the action for Vo to poll
                                let action = FOCUS_INDEX.with(|idx| {
                                    let i = *idx.borrow();
                                    FOCUSABLE_ACTIONS.with(|actions| {
                                        actions.borrow().get(i).cloned()
                                    })
                                });
                                if let Some(action) = action {
                                    PENDING_ACTIONS.with(|cell| {
                                        cell.borrow_mut().push(action);
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Process pending actions
            let actions: Vec<ActionCall> = PENDING_ACTIONS.with(|cell| {
                std::mem::take(&mut *cell.borrow_mut())
            });
            for action in actions {
                self.dispatch_action(&action);
            }
        }

        Ok(())
    }

    fn dispatch_action(&self, action: &ActionCall) {
        CURRENT_ACTION_NAME.with(|cell| {
            *cell.borrow_mut() = action.name.clone();
        });
        CURRENT_ACTION_ARGS.with(|cell| {
            *cell.borrow_mut() = action.args.clone();
        });

        let args: [u64; 0] = [];
        let mut ret: [u64; 0] = [];

        let _ = (self.call_closure_fn)(
            self.vm,
            self.fiber,
            self.on_action_closure as u64,
            args.as_ptr(),
            0,
            ret.as_mut_ptr(),
            0,
        );
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}

fn render_node(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    match node.kind.as_str() {
        "Column" => render_column(frame, area, node),
        "Row" => render_row(frame, area, node),
        "Text" => render_text(frame, area, node),
        "Button" => render_button(frame, area, node),
        "Input" => render_input(frame, area, node),
        "Spacer" => {} // Just takes space
        "Divider" => render_divider(frame, area, node),
        "Card" => render_card(frame, area, node),
        _ => {
            frame.render_widget(
                Paragraph::new(format!("[{}]", node.kind)),
                area,
            );
        }
    }
}

fn render_column(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let style_name = get_string_prop(&node.props, "style", "");
    let padding = get_int_prop(&node.props, "padding", 0) as u16;

    let inner = if padding > 0 {
        Rect {
            x: area.x + padding,
            y: area.y + padding,
            width: area.width.saturating_sub(padding * 2),
            height: area.height.saturating_sub(padding * 2),
        }
    } else {
        area
    };

    // Apply background style
    if !style_name.is_empty() {
        let block = get_style_block(&style_name);
        frame.render_widget(block, area);
    }

    let child_count = node.children.len();
    if child_count == 0 {
        return;
    }

    let spacing = get_int_prop(&node.props, "spacing", 0) as u16;

    // First pass: calculate fixed heights and count flex children
    let mut fixed_height = 0u16;
    let mut flex_count = 0u16;
    
    for child in &node.children {
        let child_height = pixel_to_rows(get_int_prop(&child.props, "height", 0) as u16);
        let flex = get_int_prop(&child.props, "flex", 0) as u16;
        
        if child_height > 0 {
            fixed_height += child_height;
        } else if flex > 0 {
            flex_count += flex;
        } else {
            fixed_height += estimate_height(child);
        }
    }
    
    // Add spacing
    if child_count > 1 {
        fixed_height += spacing * (child_count as u16 - 1);
    }
    
    // Calculate flex height
    let remaining = inner.height.saturating_sub(fixed_height);
    let flex_unit = if flex_count > 0 { remaining / flex_count } else { 0 };

    // Second pass: render children
    let mut y = inner.y;
    
    for child in &node.children {
        let child_height = pixel_to_rows(get_int_prop(&child.props, "height", 0) as u16);
        let flex = get_int_prop(&child.props, "flex", 0) as u16;
        
        let h = if child_height > 0 {
            child_height
        } else if flex > 0 {
            flex_unit * flex
        } else {
            estimate_height(child)
        };
        
        let h = h.min(inner.height.saturating_sub(y - inner.y));
        if h == 0 {
            continue;
        }

        let child_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: h,
        };

        render_node(frame, child_area, child);
        y += h + spacing;

        if y >= inner.y + inner.height {
            break;
        }
    }
}

fn render_row(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let style_name = get_string_prop(&node.props, "style", "");
    let padding = get_int_prop(&node.props, "padding", 0) as u16;

    let inner = if padding > 0 {
        Rect {
            x: area.x + padding,
            y: area.y + padding,
            width: area.width.saturating_sub(padding * 2),
            height: area.height.saturating_sub(padding * 2),
        }
    } else {
        area
    };

    // Apply background style
    if !style_name.is_empty() {
        let block = get_style_block(&style_name);
        frame.render_widget(block, area);
    }

    let child_count = node.children.len();
    if child_count == 0 {
        return;
    }

    let spacing = get_int_prop(&node.props, "spacing", 0) as u16;

    // First pass: calculate fixed widths and count flex children
    let mut fixed_width = 0u16;
    let mut flex_count = 0u16;
    
    for child in &node.children {
        let child_width = pixel_to_cols(get_int_prop(&child.props, "width", 0) as u16);
        let flex = get_int_prop(&child.props, "flex", 0) as u16;
        
        if child_width > 0 {
            fixed_width += child_width;
        } else if flex > 0 {
            flex_count += flex;
        } else {
            fixed_width += estimate_width(child);
        }
    }
    
    // Add spacing
    if child_count > 1 {
        fixed_width += spacing * (child_count as u16 - 1);
    }
    
    // Calculate flex width
    let remaining = inner.width.saturating_sub(fixed_width);
    let flex_unit = if flex_count > 0 { remaining / flex_count } else { 0 };

    // Second pass: render children
    let mut x = inner.x;

    for child in &node.children {
        let child_width = pixel_to_cols(get_int_prop(&child.props, "width", 0) as u16);
        let flex = get_int_prop(&child.props, "flex", 0) as u16;
        
        let w = if child_width > 0 {
            child_width
        } else if flex > 0 {
            flex_unit * flex
        } else {
            estimate_width(child)
        };
        
        let w = w.min(inner.width.saturating_sub(x - inner.x));
        if w == 0 {
            continue;
        }

        let child_area = Rect {
            x,
            y: inner.y,
            width: w,
            height: inner.height,
        };

        render_node(frame, child_area, child);
        x += w + spacing;

        if x >= inner.x + inner.width {
            break;
        }
    }
}

fn render_text(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let text = get_string_prop(&node.props, "text", "");
    let bold = get_bool_prop(&node.props, "bold", false);
    let color = get_string_prop(&node.props, "color", "");

    let mut style = Style::default();
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if !color.is_empty() {
        style = style.fg(parse_color(&color));
    }

    let paragraph = Paragraph::new(text).style(style);
    frame.render_widget(paragraph, area);
}

fn render_button(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let text = get_string_prop(&node.props, "text", "Button");
    let active = get_bool_prop(&node.props, "active", false);
    let style_name = get_string_prop(&node.props, "style", "");

    // Register as focusable
    if let Some(action) = node.events.get("onClick") {
        FOCUSABLE_ACTIONS.with(|cell| {
            cell.borrow_mut().push(action.clone());
        });
    }

    let is_focused = FOCUS_INDEX.with(|idx| {
        let i = *idx.borrow();
        FOCUSABLE_ACTIONS.with(|actions| i == actions.borrow().len().saturating_sub(1))
    });

    let display_text = if style_name == "icon" {
        text.clone()
    } else {
        format!("[{}]", text)
    };

    let mut style = Style::default();
    if active {
        style = style.fg(Color::Cyan);
    }
    if is_focused {
        style = style.add_modifier(Modifier::REVERSED);
    }

    let paragraph = Paragraph::new(display_text).style(style);
    frame.render_widget(paragraph, area);
}

fn render_input(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let value = get_string_prop(&node.props, "value", "");
    let placeholder = get_string_prop(&node.props, "placeholder", "");

    let display = if value.is_empty() {
        Span::styled(&placeholder, Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(&value)
    };

    let paragraph = Paragraph::new(Line::from(display))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(paragraph, area);
}

fn render_divider(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let vertical = get_bool_prop(&node.props, "vertical", false);

    if vertical {
        let line = "│".repeat(area.height as usize);
        let paragraph = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, Rect { width: 1, ..area });
    } else {
        let line = "─".repeat(area.width as usize);
        let paragraph = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, Rect { height: 1, ..area });
    }
}

fn render_card(frame: &mut Frame, area: Rect, node: &RuntimeNode) {
    let title = get_string_prop(&node.props, "title", "");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render children in inner area
    let mut y = inner.y;
    for child in &node.children {
        let h = estimate_height(child).min(inner.height.saturating_sub(y - inner.y));
        if h == 0 {
            break;
        }
        let child_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: h,
        };
        render_node(frame, child_area, child);
        y += h;
    }
}

fn get_style_block(style: &str) -> Block<'static> {
    match style {
        "sidebar" => Block::default().style(Style::default().bg(Color::Rgb(45, 45, 45))),
        "panel" => Block::default().style(Style::default().bg(Color::Rgb(37, 37, 38))),
        "editor" => Block::default().style(Style::default().bg(Color::Rgb(30, 30, 30))),
        "menubar" => Block::default().style(Style::default().bg(Color::Rgb(60, 60, 60))),
        "tabbar" => Block::default().style(Style::default().bg(Color::Rgb(45, 45, 45))),
        "statusbar" => Block::default().style(Style::default().bg(Color::Rgb(0, 122, 204)).fg(Color::White)),
        _ => Block::default(),
    }
}

fn parse_color(color: &str) -> Color {
    match color {
        "white" => Color::White,
        "black" => Color::Black,
        "gray" | "grey" => Color::DarkGray,
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        _ => Color::White,
    }
}

fn estimate_height(node: &RuntimeNode) -> u16 {
    let explicit = pixel_to_rows(get_int_prop(&node.props, "height", 0) as u16);
    if explicit > 0 {
        return explicit;
    }
    
    // Check for flex - flex elements should not contribute to fixed height calculation
    let flex = get_int_prop(&node.props, "flex", 0);
    if flex > 0 {
        return 0; // Will be calculated from remaining space
    }

    match node.kind.as_str() {
        "Text" | "Button" | "Input" => 1,
        "Divider" => 1,
        "Column" => {
            let mut h = 0u16;
            for child in &node.children {
                h = h.saturating_add(estimate_height(child));
            }
            h.max(1)
        }
        "Row" => {
            let mut max_h = 0u16;
            for child in &node.children {
                max_h = max_h.max(estimate_height(child));
            }
            max_h.max(1)
        }
        "Card" => {
            let mut h = 2u16; // borders
            for child in &node.children {
                h = h.saturating_add(estimate_height(child));
            }
            h
        }
        _ => 1,
    }
}

fn estimate_width(node: &RuntimeNode) -> u16 {
    let explicit = pixel_to_cols(get_int_prop(&node.props, "width", 0) as u16);
    if explicit > 0 {
        return explicit;
    }
    
    // Check for flex - flex elements should not contribute to fixed width calculation
    let flex = get_int_prop(&node.props, "flex", 0);
    if flex > 0 {
        return 0; // Will be calculated from remaining space
    }

    match node.kind.as_str() {
        "Text" => {
            let text = get_string_prop(&node.props, "text", "");
            text.chars().count() as u16
        }
        "Button" => {
            let text = get_string_prop(&node.props, "text", "");
            (text.chars().count() + 2) as u16 // [text]
        }
        "Input" => 20,
        "Spacer" => {
            let size = get_int_prop(&node.props, "size", 0) as u16;
            if size > 0 { size } else { 1 }
        }
        "Divider" => 1,
        "Column" | "Row" => {
            // For containers without explicit width, estimate from children
            let mut max_w = 0u16;
            for child in &node.children {
                max_w = max_w.max(estimate_width(child));
            }
            max_w.max(1)
        }
        _ => 10,
    }
}

fn get_string_prop(props: &HashMap<String, Value>, key: &str, default: &str) -> String {
    match props.get(key) {
        Some(Value::String(s)) => s.clone(),
        _ => default.to_string(),
    }
}

fn get_int_prop(props: &HashMap<String, Value>, key: &str, default: i64) -> i64 {
    match props.get(key) {
        Some(Value::Int(n)) => *n,
        Some(Value::Float(f)) => *f as i64,
        _ => default,
    }
}

// Convert pixel height to terminal rows
// TUI mode: treat pixel values as rough character units, just scale down slightly
fn pixel_to_rows(pixels: u16) -> u16 {
    if pixels == 0 { 0 } else { (pixels / 10).max(1) }
}

// Convert pixel width to terminal columns
// TUI mode: treat pixel values as rough character units, just scale down slightly  
fn pixel_to_cols(pixels: u16) -> u16 {
    if pixels == 0 { 0 } else { (pixels / 4).max(1) }
}

fn get_bool_prop(props: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    match props.get(key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

// FFI exports

fn detra_renderer_tui_run(ctx: &mut ExternCallContext) -> ExternResult {
    let title_ref = ctx.arg_ref(0);
    let width = ctx.arg_i64(1);
    let height = ctx.arg_i64(2);
    let _resizable = ctx.arg_i64(3) != 0;
    let _vsync = ctx.arg_i64(4) != 0;
    let on_action_closure = ctx.arg_ref(5);

    let _title = string::as_str(title_ref).to_string();
    let _ = (width, height); // TUI uses terminal size

    if !ctx.can_call_closure() {
        return ExternResult::Panic("detra_renderer_tui_run: closure calling not available".to_string());
    }

    let vm = ctx.vm_ptr();
    let fiber = ctx.fiber_ptr();
    let call_closure_fn = ctx.closure_call_fn().unwrap();

    let mut app = match TuiApp::new(on_action_closure, vm, fiber, call_closure_fn) {
        Ok(app) => app,
        Err(e) => return ExternResult::Panic(format!("Failed to create TUI app: {}", e)),
    };

    if let Err(e) = app.run(None) {
        return ExternResult::Panic(format!("TUI app error: {}", e));
    }

    ExternResult::Ok
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_TUI_RUN: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer-tui_Run",
    func: detra_renderer_tui_run,
};

fn detra_renderer_tui_pending_action(ctx: &mut ExternCallContext) -> ExternResult {
    let name = CURRENT_ACTION_NAME.with(|cell| cell.borrow().clone());
    ctx.ret_str(0, &name);
    ExternResult::Ok
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_TUI_PENDING_ACTION: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer-tui_PendingAction",
    func: detra_renderer_tui_pending_action,
};

fn detra_renderer_tui_pending_action_arg(ctx: &mut ExternCallContext) -> ExternResult {
    let key_ref = ctx.arg_ref(0);
    let key = string::as_str(key_ref);

    let value = CURRENT_ACTION_ARGS.with(|cell| {
        let args = cell.borrow();
        match args.get(key) {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        }
    });

    ctx.ret_str(0, &value);
    ExternResult::Ok
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_TUI_PENDING_ACTION_ARG: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer-tui_PendingActionArg",
    func: detra_renderer_tui_pending_action_arg,
};

fn detra_renderer_tui_print_once(ctx: &mut ExternCallContext) -> ExternResult {
    let width = ctx.arg_i64(0) as u16;
    let height = ctx.arg_i64(1) as u16;

    if let Some(node) = load_current_tree() {
        print_node_ascii(&node, 0, width, height);
    } else {
        println!("No current tree");
    }

    ExternResult::Ok
}

fn print_node_ascii(node: &RuntimeNode, indent: usize, _width: u16, _height: u16) {
    let prefix = "  ".repeat(indent);
    
    // Print node kind and key props
    print!("{}{}", prefix, node.kind);
    
    let mut props_str = Vec::new();
    
    // Show important props
    if let Some(Value::String(s)) = node.props.get("text") {
        let display = if s.len() > 20 { format!("\"{}...\"", &s[..17]) } else { format!("\"{}\"", s) };
        props_str.push(format!("text={}", display));
    }
    if let Some(Value::String(s)) = node.props.get("style") {
        if !s.is_empty() {
            props_str.push(format!("style=\"{}\"", s));
        }
    }
    if let Some(Value::Int(n)) = node.props.get("width") {
        props_str.push(format!("w={}", n));
    }
    if let Some(Value::Int(n)) = node.props.get("height") {
        props_str.push(format!("h={}", n));
    }
    if let Some(Value::Float(n)) = node.props.get("width") {
        props_str.push(format!("w={}", *n as i64));
    }
    if let Some(Value::Float(n)) = node.props.get("height") {
        props_str.push(format!("h={}", *n as i64));
    }
    if let Some(Value::Bool(true)) = node.props.get("fill") {
        props_str.push("fill".to_string());
    }
    match node.props.get("flex") {
        Some(Value::Float(n)) if *n > 0.0 => props_str.push(format!("flex={}", n)),
        Some(Value::Int(n)) if *n > 0 => props_str.push(format!("flex={}", n)),
        _ => {}
    }
    if let Some(Value::Bool(true)) = node.props.get("active") {
        props_str.push("active".to_string());
    }
    
    if !props_str.is_empty() {
        print!("({})", props_str.join(", "));
    }
    
    // Show events
    if !node.events.is_empty() {
        let events: Vec<_> = node.events.keys().map(|s| s.as_str()).collect();
        print!(" [{}]", events.join(", "));
    }
    
    println!();
    
    // Print children
    for child in &node.children {
        print_node_ascii(child, indent + 1, _width, _height);
    }
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_TUI_PRINT_ONCE: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer-tui_PrintOnce",
    func: detra_renderer_tui_print_once,
};

fn detra_renderer_tui_run_once(ctx: &mut ExternCallContext) -> ExternResult {
    // Config struct: Title(1) + Width(1) + Height(1) + Resizable(1) + VSync(1) = 5 slots
    let _title_ref = ctx.arg_ref(0);
    let _width = ctx.arg_i64(1);
    let _height = ctx.arg_i64(2);
    let _resizable = ctx.arg_i64(3);
    let _vsync = ctx.arg_i64(4);

    // Load current tree
    if let Some(node) = load_current_tree() {
        CURRENT_TREE.with(|cell| {
            *cell.borrow_mut() = Some(node);
        });
    }

    // Create a minimal TUI that renders once and exits
    if let Err(e) = run_once_internal() {
        return ExternResult::Panic(format!("TUI run_once error: {}", e));
    }

    ExternResult::Ok
}

fn run_once_internal() -> io::Result<()> {
    let tree = CURRENT_TREE.with(|cell| cell.borrow().clone());
    
    if let Some(tree) = tree {
        eprintln!("=== TUI Layout Preview ===");
        print_layout_preview(&tree, 0);
        eprintln!("=== End Preview ===");
    } else {
        eprintln!("TUI RunOnce: No tree loaded");
    }
    
    Ok(())
}

fn print_layout_preview(node: &RuntimeNode, indent: usize) {
    let prefix = "│ ".repeat(indent);
    let style = get_string_prop(&node.props, "style", "");
    let w = get_int_prop(&node.props, "width", 0);
    let h = get_int_prop(&node.props, "height", 0);
    
    let mut info = node.kind.clone();
    if !style.is_empty() { info.push_str(&format!(" style={}", style)); }
    if w > 0 { info.push_str(&format!(" w={}", w)); }
    if h > 0 { info.push_str(&format!(" h={}", h)); }
    
    match node.kind.as_str() {
        "Text" => {
            let text = get_string_prop(&node.props, "text", "");
            let display = if text.len() > 20 { format!("{}...", &text[..17]) } else { text };
            eprintln!("{}├─ Text \"{}\"", prefix, display);
        }
        "Button" => {
            let text = get_string_prop(&node.props, "text", "");
            let btn_style = get_string_prop(&node.props, "style", "");
            eprintln!("{}├─ Button [{}] style={}", prefix, text, btn_style);
        }
        "Input" => {
            eprintln!("{}├─ Input", prefix);
        }
        "Divider" => {
            eprintln!("{}├─ ────────", prefix);
        }
        "Spacer" => {
            eprintln!("{}├─ <spacer>", prefix);
        }
        "Column" | "Row" => {
            let dir = if node.kind == "Column" { "↓" } else { "→" };
            eprintln!("{}┌─ {} {} {}", prefix, dir, node.kind, 
                if !style.is_empty() { format!("[{}]", style) } else { String::new() });
            if w > 0 || h > 0 {
                eprintln!("{}│  ({}x{})", prefix, if w > 0 { w.to_string() } else { "auto".to_string() }, 
                    if h > 0 { h.to_string() } else { "auto".to_string() });
            }
            for child in &node.children {
                print_layout_preview(child, indent + 1);
            }
            eprintln!("{}└─", prefix);
        }
        _ => {
            eprintln!("{}├─ {}", prefix, info);
        }
    }
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_TUI_RUN_ONCE: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer-tui_RunOnce",
    func: detra_renderer_tui_run_once,
};

pub fn link_detra_renderer_tui_externs() {
    let _ = &__VO_DETRA_RENDERER_TUI_RUN;
    let _ = &__VO_DETRA_RENDERER_TUI_PENDING_ACTION;
    let _ = &__VO_DETRA_RENDERER_TUI_PENDING_ACTION_ARG;
    let _ = &__VO_DETRA_RENDERER_TUI_PRINT_ONCE;
    let _ = &__VO_DETRA_RENDERER_TUI_RUN_ONCE;
}

vo_ext::export_extensions!();
