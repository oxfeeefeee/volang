# Vibe UI System Design

A comprehensive UI framework for Vo, built on top of vo-egui.

---

## Architecture Overview

The UI system is split into two independent layers:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Applications                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐            │
│   │ vibe-studio │    │  Other App  │    │  Other App  │            │
│   └──────┬──────┘    └──────┬──────┘    └──────┬──────┘            │
│          │                  │                  │                    │
├──────────┼──────────────────┼──────────────────┼────────────────────┤
│          ▼                  │                  │                    │
│   ┌─────────────┐           │                  │    Framework Layer │
│   │   vibe-ui   │           │                  │    (optional)      │
│   │ (Elm-style) │           │                  │                    │
│   └──────┬──────┘           │                  │                    │
│          │                  │                  │                    │
├──────────┼──────────────────┼──────────────────┼────────────────────┤
│          │                  │                  │                    │
│          ▼                  ▼                  ▼                    │
│   ┌─────────────────────────────────────────────────────────────┐  │
│   │                      vo-egui                                 │  │
│   │                                                              │  │  Base Library
│   │   Low-level egui wrapper for Vo                              │  │
│   │   Independent library, anyone can use                        │  │
│   │                                                              │  │
│   └──────────────────────────┬──────────────────────────────────┘  │
│                              │                                      │
├──────────────────────────────┼──────────────────────────────────────┤
│                              ▼                                      │
│   ┌─────────────────────────────────────────────────────────────┐  │
│   │                  vo-egui-runtime (Rust)                      │  │
│   │                                                              │  │  Runtime Layer
│   │   egui + winit + wgpu + Vo FFI                               │  │
│   │                                                              │  │
│   └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Package | Purpose | Constraint |
|-------|---------|---------|------------|
| **Base Library** | `vo-egui` | Provide egui capabilities to Vo | No architecture constraints, low-level API |
| **Framework** | `vibe-ui` | Provide development patterns | Elm architecture, high-level API |
| **Application** | User code | Business logic | Uses framework or directly uses base library |

---

## Part 1: vo-egui (Base Library)

Independent library for building GUI apps in Vo. No architecture constraints.

### Package Structure

```
libs/vo-egui/
├── rust/
│   └── vo-egui-runtime/     # Rust runtime (egui + winit + wgpu)
├── vo/
│   ├── vo.mod
│   ├── app.vo               # Run, AppConfig
│   ├── widgets.vo           # Button, Label, etc.
│   ├── layout.vo            # Horizontal, Vertical, etc.
│   ├── input.vo             # TextInput, Checkbox, Slider
│   └── style.vo             # Colors, fonts, spacing
└── README.md
```

### API Style: Immediate Mode

```vo
package main

import "vo-egui"

func main() {
    checked := false
    name := ""
    count := 0
    
    egui.Run("My App", func() {
        egui.CentralPanel(func() {
            egui.Heading("Hello from Vo!")
            egui.Separator()
            
            if egui.Button("Click me") {
                count++
                println("Clicked!", count)
            }
            
            newChecked, changed := egui.Checkbox("Enable feature", checked)
            if changed {
                checked = newChecked
            }
            
            egui.Horizontal(func() {
                egui.Label("Name:")
                newName, changed := egui.TextInput(name)
                if changed {
                    name = newName
                }
            })
        })
    })
}
```

### Widget Categories

```vo
package egui

// === App Lifecycle ===
func Run(title string, frame func())
func RunWithConfig(config AppConfig, frame func())
func RequestRepaint()
func Quit()

// === Panels ===
func CentralPanel(content func())
func SidePanel(side Side, content func())
func TopPanel(content func())
func BottomPanel(content func())
func Window(title string, content func())

// === Layout ===
func Horizontal(content func())
func Vertical(content func())
func Grid(columns int, content func())
func ScrollArea(content func())
func Collapsing(header string, content func()) bool  // returns open state

// === Text ===
func Label(text string)
func Heading(text string)
func Monospace(text string)
func RichText(text string)  // supports formatting

// === Buttons ===
func Button(text string) bool              // returns clicked
func SmallButton(text string) bool
func ImageButton(imageId string) bool

// === Input Widgets ===
func Checkbox(label string, checked bool) (bool, bool)           // (newValue, changed)
func RadioButton(label string, selected bool) bool               // returns clicked
func TextInput(value string) (string, bool)                      // (newValue, changed)
func TextInputMultiline(value string) (string, bool)
func SliderInt(label string, value int, min int, max int) (int, bool)
func SliderFloat(label string, value float64, min float64, max float64) (float64, bool)
func DragInt(label string, value int) (int, bool)
func DragFloat(label string, value float64) (float64, bool)
func ColorPicker(color Color) (Color, bool)

// === Display ===
func ProgressBar(fraction float64)
func Spinner()
func Separator()
func Space(size float64)
func Image(imageId string, width float64, height float64)

// === Context ===
func AvailableWidth() float64
func AvailableHeight() float64
func IsKeyPressed(key Key) bool
func IsKeyDown(key Key) bool
```

---

## Part 2: vibe-ui (Framework)

High-level framework with Elm-style architecture. Built on vo-egui.

### Design Principles

1. **Declarative over Imperative** - Describe what UI looks like, not how to build it
2. **Unidirectional Data Flow** - State flows down, events flow up
3. **Composition over Inheritance** - Build complex UIs from simple components
4. **Async-Native** - First-class support for async operations
5. **Predictable Updates** - Clear rules for when UI re-renders

### Core Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Loop                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────┐                              ┌─────────┐     │
│   │  Model  │─────────────────────────────▶│  View   │     │
│   │ (State) │                              │  (UI)   │     │
│   └────▲────┘                              └────┬────┘     │
│        │                                        │          │
│        │                                        │ User     │
│        │                                        │ Events   │
│        │                                        ▼          │
│   ┌────┴────┐                              ┌─────────┐     │
│   │ Update  │◀─────────────────────────────│   Msg   │     │
│   │(Reducer)│                              │ (Event) │     │
│   └────┬────┘                              └─────────┘     │
│        │                                                    │
│        │ Side Effects                                       │
│        ▼                                                    │
│   ┌─────────┐        ┌─────────┐                           │
│   │   Cmd   │───────▶│ Runtime │──────▶ Msg Queue          │
│   │(Effects)│        │(Execute)│                           │
│   └─────────┘        └─────────┘                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Core Concepts

#### 1. Model (Application State)

State is a plain Vo struct. Can be nested for complex apps.

```vo
type Model struct {
    user       UserState
    todos      TodosState
    route      string
}

type UserState struct {
    loggedIn bool
    name     string
}

type TodosState struct {
    loading bool
    items   []Todo
    err     error
}
```

**Rules:**
- Model is immutable from View's perspective
- Only Update function can produce new Model
- Nested state enables component-local state

#### 2. Msg (Events/Messages)

Messages are strongly-typed structs. Use type switch for dispatch.

```vo
// Msg type is `any` - any struct can be a message
// This follows Bubble Tea's approach (Go's popular TUI framework)

// UI Events
type ButtonClicked struct{ id string }
type InputChanged struct{ id string; value string }
type FormSubmitted struct{}

// Async Events  
type UsersFetched struct{ users []User }
type FetchFailed struct{ err error }

// Navigation
type Navigate struct{ path string }

// Domain Events (naming convention: {Domain}{Action})
type TodoAdd struct{ text string }
type TodoToggle struct{ id int }
type TodoDelete struct{ id int }
```

#### 3. Update (State Reducer)

Pure function: `(Model, any) -> (Model, Cmd)`. Returns new state and optional side effects.

```vo
func update(model Model, msg any) (Model, Cmd) {
    switch m := msg.(type) {
    case TodoAdd:
        newTodos := append(model.todos.items, Todo{text: m.text})
        return model.withTodos(newTodos), cmd.None
    
    case TodoToggle:
        // ... toggle logic
        return newModel, cmd.None
    
    case FetchUsers:
        return model.withLoading(true), 
               cmd.Http("GET", "/api/users", nil).
                   OnDone(func(status int, body []byte, err error) any {
                       if err != nil {
                           return FetchFailed{err}
                       }
                       return UsersFetched{parseUsers(body)}
                   })
    
    case UsersFetched:
        return model.withUsers(m.users).withLoading(false), cmd.None
    
    case Navigate:
        return model.withRoute(m.path), cmd.None
    }
    return model, cmd.None
}
```

#### 4. Cmd (Side Effects)

Commands are **descriptions** of side effects, not the effects themselves. Runtime executes them.

```vo
package cmd

// Cmd is an opaque struct, not a function
type Cmd struct {
    // internal fields managed by framework
}

// === Basic ===
var None Cmd                                    // No operation
func Msg(m any) Cmd                             // Send a message immediately
func Batch(cmds ...Cmd) Cmd                     // Execute multiple commands in parallel
func Sequence(cmds ...Cmd) Cmd                  // Execute commands in order

// === Async ===
func Task(fn func() any) Cmd                    // Run function in background
func After(d time.Duration, msg any) Cmd        // Delayed message

// === HTTP ===
func Http(method string, url string, body []byte) HttpBuilder

type HttpBuilder struct{}
func (b HttpBuilder) Header(key string, value string) HttpBuilder
func (b HttpBuilder) OnDone(fn func(status int, body []byte, err error) any) Cmd

// === Navigation ===
func Navigate(path string) Cmd
func NavigateBack() Cmd

// === UI Control ===
func Focus(elementId string) Cmd
func CopyToClipboard(text string) Cmd

// === Cancellation ===
func Cancel(id string) Cmd                      // Cancel a command by ID

// === Cmd Configuration (chaining) ===
func (c Cmd) WithId(id string) Cmd              // Assign ID for cancellation
func (c Cmd) OnCancel(msg any) Cmd              // Message to send when cancelled
func (c Cmd) Timeout(d time.Duration) Cmd       // Set timeout
func (c Cmd) OnTimeout(msg any) Cmd             // Message to send on timeout
```

**Usage Example:**

```vo
// Cancelable HTTP request with timeout
cmd.Http("GET", "/api/data", nil).
    OnDone(func(status int, body []byte, err error) any {
        if err != nil {
            return FetchFailed{err}
        }
        return DataFetched{body}
    }).
    WithId("fetch-data").
    OnCancel(FetchCancelled{}).
    Timeout(10 * time.Second).
    OnTimeout(FetchTimeout{})

// Later, to cancel:
cmd.Cancel("fetch-data")
```

#### 5. View (UI Description)

View is a pure function that returns a UI tree. Uses vo-egui under the hood.

```vo
func view(model Model) {
    vibe.CentralPanel(func() {
        vibe.Heading("My App")
        vibe.Separator()
        
        // Conditional rendering
        if model.user.loggedIn {
            userPanel(model.user)
        } else {
            loginForm()
        }
        
        // List rendering
        for i, todo := range model.todos.items {
            todoItem(todo, i)
        }
        
        // Button with message
        if vibe.Button("Add Todo") {
            vibe.Send(TodoAdd{text: "New item"})
        }
    })
}
```

#### 6. Components (Reusable Views)

Components are just functions. They can send messages via `vibe.Send()`.

```vo
func todoItem(todo Todo, index int) {
    vibe.Horizontal(func() {
        checked, changed := vibe.Checkbox("", todo.done)
        if changed {
            vibe.Send(TodoToggle{index})
        }
        
        vibe.Label(todo.text)
        
        if vibe.Button("×") {
            vibe.Send(TodoDelete{index})
        }
    })
}

func counter(value int, onIncr any, onDecr any) {
    vibe.Horizontal(func() {
        if vibe.Button("-") {
            vibe.Send(onDecr)
        }
        vibe.Label(fmt.Sprint(value))
        if vibe.Button("+") {
            vibe.Send(onIncr)
        }
    })
}
```

---

## Threading Model

### Constraints

1. **egui/winit requirement** - UI event loop must run on main thread (macOS/Windows)
2. **Vo VM** - Single-threaded interpreter with cooperative goroutine scheduler
3. **Async Cmd** - Must not block UI, needs background execution
4. **Frame rate** - 60fps = 16ms per frame, no blocking allowed

### Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                              Process                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                     Main Thread (OS)                          │ │
│  │                                                               │ │
│  │   ┌────────────┐    ┌────────────┐    ┌────────────┐         │ │
│  │   │   winit    │───▶│   egui     │───▶│   wgpu     │         │ │
│  │   │ EventLoop  │    │  Context   │    │  Render    │         │ │
│  │   └─────┬──────┘    └─────┬──────┘    └────────────┘         │ │
│  │         │                 │                                   │ │
│  │         ▼                 ▼                                   │ │
│  │   ┌─────────────────────────────────────────────────────┐    │ │
│  │   │              Vibe Runtime (Rust)                     │    │ │
│  │   │                                                      │    │ │
│  │   │   ┌──────────┐  ┌──────────┐  ┌──────────┐          │    │ │
│  │   │   │   Msg    │  │   Cmd    │  │  State   │          │    │ │
│  │   │   │  Queue   │  │ Executor │  │  Store   │          │    │ │
│  │   │   └────┬─────┘  └────┬─────┘  └──────────┘          │    │ │
│  │   │        │             │                               │    │ │
│  │   └────────┼─────────────┼───────────────────────────────┘    │ │
│  │            │             │                                    │ │
│  │            ▼             │                                    │ │
│  │   ┌─────────────────┐    │                                    │ │
│  │   │     Vo VM       │    │                                    │ │
│  │   │                 │    │                                    │ │
│  │   │  update()       │◀───┘  (Cmd completion sends Msg)        │ │
│  │   │  view()         │                                         │ │
│  │   │                 │                                         │ │
│  │   └─────────────────┘                                         │ │
│  │                                                               │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                   Worker Threads (Async Cmds)                  │ │
│  │                                                                │ │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐          │ │
│  │   │  HTTP   │  │  File   │  │  Timer  │  │  ...    │          │ │
│  │   │ Request │  │   I/O   │  │         │  │         │          │ │
│  │   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘          │ │
│  │        │            │            │            │                │ │
│  │        └────────────┴────────────┴────────────┘                │ │
│  │                           │                                    │ │
│  │                           ▼                                    │ │
│  │                    Msg Queue (thread-safe)                     │ │
│  │                                                                │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Thread Responsibilities

| Thread | Responsibility | Blocking Tolerance |
|--------|----------------|-------------------|
| **Main Thread** | UI event loop, rendering, Vo VM execution | ❌ Cannot block >16ms |
| **Worker Threads** | HTTP, file I/O, compute-intensive tasks | ✅ Can block |

### Frame Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Frame (~16ms budget)                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Process OS Events (winit)                      ~1ms    │
│     - Mouse, keyboard, window events                        │
│     - Convert to Msg, push to queue                         │
│                                                             │
│  2. Drain Msg Queue                                ~2ms    │
│     while msg = queue.poll():                              │
│         model, cmd = update(model, msg)   // Vo VM         │
│         schedule(cmd)                     // async exec    │
│                                                             │
│  3. Execute Sync Cmds                              ~1ms    │
│     - cmd.Msg(m) → push to queue immediately               │
│     - cmd.Focus(id) → execute immediately                  │
│                                                             │
│  4. Build UI                                       ~5ms    │
│     view(model)                           // Vo VM         │
│                                                             │
│  5. Render                                         ~5ms    │
│     egui.render()                         // Rust          │
│                                                             │
│  6. Check Async Cmd Completions                    ~1ms    │
│     - Worker threads push Msg to queue on completion       │
│     - If new Msg, trigger next frame                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

**1. Vo VM on Main Thread**

```
Why:
- Avoids multi-thread synchronization complexity
- Model/View/Update are naturally sequential
- egui is immediate-mode, needs synchronous access

Trade-off:
- update() and view() cannot be slow
- Heavy computation must be offloaded to Cmd
```

**2. Msg Queue Design**

```vo
// Thread-safe message queue (Rust implementation)
type MsgQueue struct {
    // Internally uses crossbeam or std::sync::mpsc
}

// Worker thread submits message
func (q *MsgQueue) Push(msg any)

// Main thread drains messages
func (q *MsgQueue) Drain() []any
```

**3. Cmd Cancellation**

```
Runtime tracks active Cmds by ID:
  activeCmds: map[id] -> CancelToken

When Cancel(id) is called:
  1. Look up CancelToken by id
  2. Signal cancellation
  3. Send OnCancel message if configured
  4. Remove from active map
```

---

## Application Entry Point

```vo
package main

import "vibe-ui"

func main() {
    app := vibe.App{
        Init:   initApp,
        Update: update,
        View:   view,
    }
    
    vibe.Run(vibe.Config{
        Title:  "My App",
        Width:  1200,
        Height: 800,
    }, app)
}

func initApp() (Model, cmd.Cmd) {
    return Model{
        route: "/",
        todos: TodosState{loading: true},
    }, cmd.Http("GET", "/api/todos", nil).
           OnDone(parseTodosResponse)
}
```

---

## Async Patterns

### Loading States

Since Vo has no generics, define per-type async state:

```vo
type TodosState struct {
    loading bool
    items   []Todo
    err     error
}

type UserState struct {
    loading bool
    user    User
    err     error
}

// Usage in view
func todosView(state TodosState) {
    if state.loading {
        vibe.Spinner()
        return
    }
    if state.err != nil {
        vibe.Label("Error: " + state.err.Error())
        return
    }
    for _, todo := range state.items {
        todoItem(todo)
    }
}
```

### Debouncing

```vo
case SearchInput:
    return model.withQuery(m.query), 
           cmd.Debounce("search", 300*time.Millisecond, DoSearch{m.query})
```

---

## Subscriptions (External Events)

```vo
func subscriptions(model Model) []vibe.Sub {
    subs := []vibe.Sub{}
    
    // Keyboard shortcuts
    subs = append(subs, vibe.OnKeyDown(func(key vibe.Key) any {
        if key.Ctrl && key.Code == "s" {
            return SaveDocument{}
        }
        return nil
    }))
    
    // Window resize
    subs = append(subs, vibe.OnResize(func(w, h int) any {
        return WindowResized{w, h}
    }))
    
    // Interval timer
    if model.autoSaveEnabled {
        subs = append(subs, vibe.Every(30*time.Second, AutoSave{}))
    }
    
    return subs
}
```

---

## Testing

```vo
func TestUpdate(t *testing.T) {
    model := Model{count: 0}
    
    newModel, c := update(model, Increment{})
    
    assert(newModel.count == 1)
    assert(c == cmd.None)
}
```

---

## Implementation Phases

### Phase 1: vo-egui (Base Library)
- [ ] Rust runtime with egui + winit + wgpu
- [ ] Basic widget externs (Button, Label, Checkbox, etc.)
- [ ] Layout containers (Horizontal, Vertical, Panel)
- [ ] App lifecycle (Run, Quit, RequestRepaint)

### Phase 2: vibe-ui Core
- [ ] Model/Update/View loop
- [ ] Msg queue and dispatch
- [ ] Basic Cmd (None, Batch, Msg)
- [ ] vibe.Send() for view-to-runtime messaging

### Phase 3: Async & Effects
- [ ] cmd.Http with cancellation
- [ ] cmd.Task for background work
- [ ] cmd.After (delayed messages)
- [ ] Subscriptions

### Phase 4: Advanced Features
- [ ] Routing / Navigation
- [ ] Component-scoped Cmd cancellation
- [ ] DevTools (state inspector)
- [ ] Hot reload
