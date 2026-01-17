//! RenderTree-based rendering - simple and direct.

use eframe::egui;
use detra_renderable::{RenderTree, RenderNode, RenderKind, Color, Rect};

use super::PENDING_ACTIONS;

/// Render a RenderTree to egui and handle input.
pub fn render(ctx: &egui::Context, tree: &RenderTree) {
    // 1. Handle input using hit_test
    handle_input(ctx, tree);
    
    // 2. Paint all nodes
    let painter = ctx.layer_painter(egui::LayerId::background());
    paint_node(&painter, &tree.root);
}

/// Handle input events using hit_test
fn handle_input(ctx: &egui::Context, tree: &RenderTree) {
    ctx.input(|input| {
        if input.pointer.any_released() {
            if let Some(pos) = input.pointer.interact_pos() {
                if let Some(node) = tree.hit_test(pos.x, pos.y) {
                    if let Some(action) = &node.on_click {
                        PENDING_ACTIONS.with(|cell| {
                            cell.borrow_mut().push(action.clone());
                        });
                    }
                }
            }
        }
    });
}

fn paint_node(painter: &egui::Painter, node: &RenderNode) {
    if !node.visible {
        return;
    }
    
    let rect = to_egui_rect(&node.rect);
    
    match &node.kind {
        RenderKind::Container { background, border } => {
            if let Some(bg) = background {
                painter.rect_filled(rect, 0.0, to_egui_color(bg));
            }
            if let Some(border) = border {
                painter.rect_stroke(
                    rect,
                    border.radius,
                    egui::Stroke::new(border.width, to_egui_color(&border.color)),
                );
            }
        }
        
        RenderKind::Text { content, style } => {
            let font = if style.monospace {
                egui::FontId::monospace(style.size)
            } else {
                egui::FontId::proportional(style.size)
            };
            
            painter.text(
                rect.min,
                egui::Align2::LEFT_TOP,
                content,
                font,
                to_egui_color(&style.color),
            );
        }
        
        RenderKind::Button { text, style, background, active, .. } => {
            let bg = if *active {
                egui::Color32::from_rgb(background.r.saturating_add(20), background.g.saturating_add(20), background.b.saturating_add(20))
            } else {
                to_egui_color(background)
            };
            
            painter.rect_filled(rect, 3.0, bg);
            
            let font = egui::FontId::proportional(style.size);
            let text_pos = egui::pos2(rect.min.x + 8.0, rect.center().y - style.size / 2.0);
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                text,
                font,
                to_egui_color(&style.color),
            );
        }
        
        RenderKind::Input { value, placeholder, style, background, .. } => {
            painter.rect_filled(rect, 3.0, to_egui_color(background));
            painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)));
            
            let text = if value.is_empty() { placeholder } else { value };
            let color = if value.is_empty() {
                egui::Color32::from_rgb(128, 128, 128)
            } else {
                to_egui_color(&style.color)
            };
            
            let font = if style.monospace {
                egui::FontId::monospace(style.size)
            } else {
                egui::FontId::proportional(style.size)
            };
            
            painter.text(
                egui::pos2(rect.min.x + 4.0, rect.min.y + 4.0),
                egui::Align2::LEFT_TOP,
                text,
                font,
                color,
            );
        }
        
        RenderKind::Divider { color, vertical } => {
            if *vertical {
                painter.line_segment(
                    [egui::pos2(rect.center().x, rect.min.y), egui::pos2(rect.center().x, rect.max.y)],
                    egui::Stroke::new(1.0, to_egui_color(color)),
                );
            } else {
                painter.line_segment(
                    [egui::pos2(rect.min.x, rect.center().y), egui::pos2(rect.max.x, rect.center().y)],
                    egui::Stroke::new(1.0, to_egui_color(color)),
                );
            }
        }
        
        RenderKind::Spacer => {
            // Invisible
        }
        
        RenderKind::Image { src } => {
            // TODO: Image loading
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(60, 60, 60));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("[{}]", src),
                egui::FontId::proportional(10.0),
                egui::Color32::GRAY,
            );
        }
    }
    
    // Paint children
    for child in &node.children {
        paint_node(painter, child);
    }
}

fn to_egui_rect(rect: &Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(rect.x, rect.y),
        egui::vec2(rect.width, rect.height),
    )
}

fn to_egui_color(color: &Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}
