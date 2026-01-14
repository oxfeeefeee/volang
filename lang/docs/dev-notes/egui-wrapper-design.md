# Egui Wrapper Design (Vo)

## Goals

- **Easy to use** from Vo.
- **High performance**: event-driven repaint, no busy loops, and a clear path to batching.
- **Async-first** application model in Vo to avoid common immediate-mode UI pain points:
  - state scattered across widgets
  - async I/O mixed into drawing code
  - uncontrolled repaint storms

## Non-goals (MVP)

- Multi-window support.
- Mobile targets.
- JIT support for the UI runtime on wasm.

## Repo Placement

- Vo package:
  - `libs/ui-egui/vo` (Vo-facing API)
- Rust runtime:
  - `libs/ui-egui/rust/vo-egui-runtime` (egui + wgpu runtime + extern surface)
- Product integration:
  - `vibe-studio` (loads/runs a Vo UI app)

## High-level Architecture

The wrapper is split into two layers:

1. **Rust UI runtime** (`vo-egui-runtime`)
   - Owns the platform event loop (desktop) or browser runner (web).
   - Owns rendering backend (wgpu).
   - Calls back into Vo each frame using the existing closure-callback ABI (`ExternCallContext::call_closure`).

2. **Vo UI framework** (`libs/ui-egui/vo`)
   - Provides an async-friendly application model.
   - Provides UI functions that map onto egui via extern calls.
   - Centralizes state updates and repaint scheduling.

## Platform Targets

### Desktop

- Backend: `winit + egui + wgpu`.
- Model: Rust drives `EventLoop`, calls into Vo `frame` closure.

### Web

- MVP plan: Vo VM runs in wasm (interpreter mode). Rust runner drives egui with a browser animation frame loop.
- `Handle.Wait()` is not supported on web (must fail).

## Vo Application Model (Async-friendly)

### Core idea

Use a unidirectional data flow:

- **Model**: pure data (application state)
- **Msg**: events/messages (UI events, async completions, timers)
- **Update(model, msg) -> model**: the only place to mutate model
- **View(model)**: drawing only (no I/O)

Async work produces messages; the UI thread drains messages in batches and triggers repaint only when needed.

### Public API surface (draft)

- `Start(opts, app) -> Handle`
- `Handle.Wait()` (desktop only)
- `Handle.Stop()`
- `Handle.Post(msg)`

Runtime hooks exposed to app:

- `rt.Dispatch(msg)`
- `rt.Spawn(task -> msg)`
- `rt.RequestRepaint()`
- `rt.RequestRepaintAfter(ms)`

## Mapping egui to Vo: Scope-closure pattern

To keep the Vo API ergonomic and avoid passing `Ui*` handles everywhere, we use scope calls that execute a Vo closure inside a Rust egui scope.

Example (conceptual):

- Vo: `ui.Window(title, fn() { ... })`
- Rust: `egui::Window::new(title).show(ctx, |ui| { call_vo_closure(body) })`

Widgets called from Vo inside that scope operate on the current egui `Ui` stored in a Rust TLS slot.

### MVP extern set

- `egui_run(opts, frame_closure)`
- Scopes:
  - `egui_window(title, body_closure)`
  - `egui_vertical(body_closure)`
  - `egui_horizontal(body_closure)`
  - `egui_id_scope(key, body_closure)`
- Widgets:
  - `egui_label(text)`
  - `egui_button(text) -> bool`
  - `egui_checkbox(label, checked) -> bool`
  - `egui_text_edit_singleline(text) -> (newText, changed)`
  - `egui_slider_int(label, val, min, max) -> (newVal, changed)`
  - `egui_slider_float(label, val, min, max) -> (newVal, changed)`

## Callback ABI: calling Vo closures from Rust extern

The runtime already supports calling Vo closures from extern functions:

- Rust extern receives `ExternCallContext` which contains:
  - `vm` pointer
  - `fiber` pointer
  - `call_closure_fn`

The egui runtime uses these to call the Vo frame closure each frame.

## Repaint Policy (Performance)

- Default to **event-driven rendering**.
- Only repaint when:
  - input events arrived
  - message queue is non-empty
  - animations/timers request repaint

The Vo runtime layer batches pending messages per frame and can coalesce redundant updates.

## Threading Model

- Desktop: UI runs on the main OS thread.
- Vo async tasks run on the same thread using the VM scheduler (cooperative), and must not block.

## Error Handling

- Unrecoverable configuration/programming errors should fail fast.
- UI runtime errors should surface as Vo errors (where applicable) or as a panic with a clear message.

## Staged Implementation Plan

1. Desktop runner + minimal extern surface (`egui_run`, window/layout, label/button)
2. Vo high-level API (`Start/Handle`, message queue, repaint)
3. Integrate into `vibe-studio` and provide a minimal demo app
4. Web runner (wasm) and packaging flow

