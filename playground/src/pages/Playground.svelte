<script lang="ts">
  import Editor from '../components/Editor.svelte';
  import Output from '../components/Output.svelte';
  import FileExplorer from '../components/FileExplorer.svelte';
  import GuiPreview from '../components/GuiPreview.svelte';
  import { runCode, initGuiApp, handleGuiEvent, setRenderCallback, type RunStatus } from '../wasm/vo.ts';

  let code = $state(`// Tetris Game
package main

import "gui"

// ============ Constants ============

const Rows = 20
const Cols = 10
const CellSize = 25

// ============ Types ============

type Point struct {
	X int
	Y int
}

type Piece struct {
	Type  int // 0-6: I, J, L, O, S, T, Z
	Rot   int // 0-3
	X     int
	Y     int
	Cells []Point // Relative coordinates
}

type State struct {
	Grid      []int // Rows * Cols, 0=empty, >0=colorIndex
	Score     int
	Level     int
	GameOver  bool
	Paused    bool
	Current   Piece
	TimerID   int
	TickCount int
    RngState  int
}

// Colors for pieces: 0=Empty, 1=Cyan, 2=Blue, 3=Orange, 4=Yellow, 5=Green, 6=Purple, 7=Red
var colors = []string{
	"#1a1a2e", // Background
	"#00f0f0", // I - Cyan
	"#0000f0", // J - Blue
	"#f0a000", // L - Orange
	"#f0f000", // O - Yellow
	"#00f000", // S - Green
	"#a000f0", // T - Purple
	"#f00000", // Z - Red
}

// Shapes: [Type][Rotation][4 cells]
// Coordinates are relative to pivot
// I: Type 0
var shapeI = []Point{Point{-1, 0}, Point{0, 0}, Point{1, 0}, Point{2, 0}}
// J: Type 1
var shapeJ = []Point{Point{-1, -1}, Point{-1, 0}, Point{0, 0}, Point{1, 0}}
// L: Type 2
var shapeL = []Point{Point{1, -1}, Point{-1, 0}, Point{0, 0}, Point{1, 0}}
// O: Type 3
var shapeO = []Point{Point{0, 0}, Point{1, 0}, Point{0, 1}, Point{1, 1}}
// S: Type 4
var shapeS = []Point{Point{0, 0}, Point{1, 0}, Point{-1, 1}, Point{0, 1}}
// T: Type 5
var shapeT = []Point{Point{0, 0}, Point{-1, 0}, Point{1, 0}, Point{0, -1}}
// Z: Type 6
var shapeZ = []Point{Point{-1, 0}, Point{0, 0}, Point{0, 1}, Point{1, 1}}

// ============ App ============

func main() {
    gui.SetGlobalKeyHandler(handleKey)
	gui.Run(gui.App{
		Init: initGame,
		View: view,
	})
}

func initGame() any {
	s := &State{
		Grid:     make([]int, Rows*Cols),
		Score:    0,
		Level:    1,
		GameOver: false,
		Paused:   false,
        RngState: 12345, // Seed
	}
	spawnPiece(s)
	// Start timer: 500ms
	s.TimerID = gui.SetInterval(500, func() {
		tick(s)
	})
	return s
}

// ============ Update Logic ============

func tick(s *State) {
	if s.GameOver || s.Paused {
		return
	}
	
	// Move down
	if canMove(s, s.Current, 0, 1) {
		s.Current.Y++
	} else {
		lockPiece(s)
		clearLines(s)
		spawnPiece(s)
		if !canMove(s, s.Current, 0, 0) {
			s.GameOver = true
		}
	}
}

func handleKey(state any, key string) {
	s := state.(*State)
	if s.GameOver {
        if key == "Enter" {
            restart(s)
        }
		return
	}
    
    if key == "p" || key == "P" {
        s.Paused = !s.Paused
        return
    }

	if s.Paused {
		return
	}

	switch key {
	case "ArrowLeft":
		if canMove(s, s.Current, -1, 0) {
			s.Current.X--
		}
	case "ArrowRight":
		if canMove(s, s.Current, 1, 0) {
			s.Current.X++
		}
	case "ArrowDown":
		if canMove(s, s.Current, 0, 1) {
			s.Current.Y++
		}
	case "ArrowUp":
		rotatePiece(s)
    case " ":
        dropPiece(s)
	}
}

func restart(s *State) {
    s.Grid = make([]int, Rows*Cols)
    s.Score = 0
    s.Level = 1
    s.GameOver = false
    s.Paused = false
    spawnPiece(s)
}

func spawnPiece(s *State) {
    // Random type 0-6
    t := rand(s) % 7
    s.Current = Piece{
        Type: t,
        Rot: 0,
        X: Cols / 2 - 1,
        Y: 0,
        Cells: getCells(t, 0),
    }
}

func getCells(t int, rot int) []Point {
    var base []Point
    if t == 0 { base = shapeI }
    if t == 1 { base = shapeJ }
    if t == 2 { base = shapeL }
    if t == 3 { base = shapeO }
    if t == 4 { base = shapeS }
    if t == 5 { base = shapeT }
    if t == 6 { base = shapeZ }
    
    // Rotate
    cells := make([]Point, 4)
    for i, p := range base {
        x, y := p.X, p.Y
        for r := 0; r < rot; r++ {
            // Rotate 90 deg clockwise: (x, y) -> (-y, x)
            x, y = -y, x
        }
        cells[i] = Point{x, y}
    }
    return cells
}

func rotatePiece(s *State) {
    newRot := (s.Current.Rot + 1) % 4
    newCells := getCells(s.Current.Type, newRot)
    
    // Test if valid
    p := s.Current
    p.Rot = newRot
    p.Cells = newCells
    
    if canMove(s, p, 0, 0) {
        s.Current = p
    } else {
        // Wall kicks (simple)
        if canMove(s, p, 1, 0) {
            s.Current = p
            s.Current.X++
        } else if canMove(s, p, -1, 0) {
            s.Current = p
            s.Current.X--
        }
    }
}

func canMove(s *State, p Piece, dx int, dy int) bool {
    for _, cell := range p.Cells {
        nx := p.X + cell.X + dx
        ny := p.Y + cell.Y + dy
        
        if nx < 0 || nx >= Cols || ny >= Rows {
            return false
        }
        if ny >= 0 {
            idx := ny*Cols + nx
            if s.Grid[idx] != 0 {
                return false
            }
        }
    }
    return true
}

func lockPiece(s *State) {
    c := s.Current.Type + 1
    for _, cell := range s.Current.Cells {
        nx := s.Current.X + cell.X
        ny := s.Current.Y + cell.Y
        if ny >= 0 && ny < Rows && nx >= 0 && nx < Cols {
            s.Grid[ny*Cols+nx] = c
        }
    }
}

func clearLines(s *State) {
    lines := 0
    for y := Rows - 1; y >= 0; y-- {
        full := true
        for x := 0; x < Cols; x++ {
            if s.Grid[y*Cols+x] == 0 {
                full = false
                break
            }
        }
        if full {
            lines++
            // Move lines down
            for ky := y; ky > 0; ky-- {
                for kx := 0; kx < Cols; kx++ {
                    s.Grid[ky*Cols+kx] = s.Grid[(ky-1)*Cols+kx]
                }
            }
            // Clear top line
            for kx := 0; kx < Cols; kx++ {
                s.Grid[kx] = 0
            }
            y++ // Recheck this line
        }
    }
    if lines > 0 {
        s.Score += lines * 100 * s.Level
    }
}

func dropPiece(s *State) {
    for canMove(s, s.Current, 0, 1) {
        s.Current.Y++
    }
    lockPiece(s)
    clearLines(s)
    spawnPiece(s)
    if !canMove(s, s.Current, 0, 0) {
        s.GameOver = true
    }
}

// Pseudo-random number generator
func rand(s *State) int {
    s.RngState = (s.RngState * 1103515245 + 12345) % 2147483648
    if s.RngState < 0 {
        s.RngState = -s.RngState
    }
    return s.RngState
}

// ============ View ============

func view(state any) gui.Node {
	s := state.(*State)
    
	return gui.Center(
		gui.Column(
            // Header
			gui.Row(
				gui.H2("TETRIS").Fg("#a000f0"),
				gui.Spacer(),
				gui.Column(
                    gui.Text("Score: ", s.Score).Font(16).Bold(),
                    gui.Text("Level: ", s.Level).Font(12),
                ),
			).W(CellSize * Cols).M(10),
            
            // Game Board
            gui.Column(
                renderGrid(s),
            ).Bg("#000").Border("2px solid #333").P(0),
            
            // Controls Info
            gui.Text("Controls: Arrows to move/rotate, Space to drop").Font(12).Fg("#666").M(10),
            
            // Game Over Overlay
            gui.Show(s.GameOver, 
                gui.Center(
                    gui.Column(
                        gui.H2("GAME OVER").Fg("#f00"),
                        gui.Text("Final Score: ", s.Score),
                        gui.Button("Restart", gui.On(actionRestart)).Bg("#a000f0").Fg("#fff"),
                    ).P(20).Bg("rgba(0,0,0,0.8)").Rounded(10),
                ).Style(map[string]any{
                    "position": "absolute", 
                    "top": 0, "left": 0, "right": 0, "bottom": 0,
                }),
            ),
		).Style(map[string]any{"position": "relative"}),
	)
}

func renderGrid(s *State) gui.Node {
    var cells []gui.Node
    
    // Fill with background grid
    displayGrid := make([]int, Rows*Cols)
    copy(displayGrid, s.Grid)
    
    // Draw current piece
    if !s.GameOver {
        c := s.Current.Type + 1
        for _, cell := range s.Current.Cells {
            nx := s.Current.X + cell.X
            ny := s.Current.Y + cell.Y
            if ny >= 0 && ny < Rows && nx >= 0 && nx < Cols {
                displayGrid[ny*Cols+nx] = c
            }
        }
    }
    
    for i := 0; i < Rows*Cols; i++ {
        colorIdx := displayGrid[i]
        color := colors[colorIdx]
        cells = append(cells, 
            gui.Node{Type: "Block"}.
                W(CellSize).H(CellSize).
                Bg(color).
                Border("1px solid rgba(255,255,255,0.1)"),
        )
    }
    
    return gui.Grid(Cols, cells...).Gap(0)
}

func actionRestart(s *State) {
    restart(s)
}

func copy(dst []int, src []int) {
    for i := 0; i < len(src); i++ {
        dst[i] = src[i]
    }
}`);

  let stdout = $state('');
  let stderr = $state('');
  let status: RunStatus = $state('idle');
  let currentFile = $state('gui_tetris.vo');
  let guiMode = $state(false);
  let nodeTree: any = $state(null);
  let consoleCollapsed = $state(false);

  // Register render callback for async updates (timers)
  setRenderCallback((json: string) => {
    try {
      const parsed = JSON.parse(json);
      nodeTree = parsed.tree;
    } catch (e) {
      console.error('Failed to parse render JSON from timer:', e);
    }
  });

  async function handleRun() {
    status = 'running';
    stdout = '';
    stderr = '';
    nodeTree = null;
    guiMode = false;

    // Detect GUI code by checking for gui.Run
    const isGuiCode = code.includes('gui.Run') && code.includes('import "gui"');

    try {
      if (isGuiCode) {
        // Use initGuiApp for GUI code
        const result = await initGuiApp(code);
        console.log('initGuiApp result:', result);
        if (result.status !== 'ok') {
          stderr = result.error || 'Unknown error';
          status = 'error';
          return;
        }
        guiMode = true;
        console.log('renderJson:', result.renderJson);
        if (result.renderJson) {
          const parsed = JSON.parse(result.renderJson);
          console.log('parsed tree:', parsed);
          nodeTree = parsed.tree;
        }
        status = 'success';
      } else {
        // Regular code execution
        const result = await runCode(code);
        let output = result.stdout;
        
        // Check for VoGUI output (legacy path)
        if (output.startsWith('__VOGUI__')) {
          guiMode = true;
          const jsonStr = output.slice(9).trim();
          try {
            const parsed = JSON.parse(jsonStr);
            nodeTree = parsed.tree;
            stdout = '';
          } catch (parseErr) {
            stderr = 'Failed to parse GUI output: ' + parseErr;
            status = 'error';
            return;
          }
        } else {
          stdout = output;
        }
        
        stderr = result.stderr;
        status = result.status === 'ok' ? 'success' : 'error';
      }
    } catch (e) {
      stderr = e instanceof Error ? e.message : String(e);
      status = 'error';
    }
  }

  async function onGuiEvent(handlerId: number, payload: string) {
    console.log('[Playground] onGuiEvent:', handlerId, payload);
    try {
      const result = await handleGuiEvent(handlerId, payload);
      console.log('[Playground] handleGuiEvent result:', result.status, result.renderJson?.length || 0);
      if (result.status !== 'ok') {
        stderr = result.error;
        return;
      }
      if (result.renderJson) {
        const parsed = JSON.parse(result.renderJson);
        nodeTree = parsed.tree;
      }
    } catch (e) {
      stderr = e instanceof Error ? e.message : String(e);
    }
  }

  function handleReset() {
    stdout = '';
    stderr = '';
    status = 'idle';
    guiMode = false;
    nodeTree = null;
  }

  function handleFileSelect(content: string, filename: string) {
    code = content;
    currentFile = filename;
    handleReset();
  }
</script>

<div class="playground">
  <div class="toolbar">
    <div class="toolbar-left">
      <div class="actions">
        <button class="btn-primary" onclick={handleRun} disabled={status === 'running'}>
          {status === 'running' ? 'Running...' : 'Run'}
        </button>
        <button class="btn-secondary" onclick={handleReset}>
          Reset
        </button>
      </div>
      {#if guiMode}
        <span class="mode-badge gui">GUI Mode</span>
      {/if}
      {#if currentFile}
        <span class="current-file">{currentFile}</span>
      {/if}
    </div>
  </div>

  <div class="main-area">
    <div class="editor-row">
      <div class="sidebar">
        <FileExplorer onSelect={handleFileSelect} bind:selectedFile={currentFile} />
      </div>
      <div class="editor-panel">
        <Editor bind:value={code} />
      </div>
      {#if guiMode}
        <div class="gui-panel">
          <GuiPreview {nodeTree} interactive={true} onEvent={onGuiEvent} />
        </div>
      {/if}
    </div>
    <div class="console-panel" class:collapsed={consoleCollapsed && guiMode}>
      <Output {stdout} {stderr} {status} collapsible={guiMode} bind:collapsed={consoleCollapsed} />
    </div>
  </div>
</div>

<style>
  .playground {
    display: flex;
    flex-direction: column;
    height: calc(100vh - var(--header-height));
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    padding: 10px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .mode-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 4px;
    font-weight: 600;
  }

  .mode-badge.gui {
    background: var(--accent);
    color: white;
  }

  .current-file {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    padding: 4px 10px;
    background: var(--bg-tertiary);
    border-radius: 4px;
  }

  .main-area {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .editor-row {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 280px;
    min-width: 200px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .editor-panel {
    flex: 1;
    border-right: 1px solid var(--border);
    overflow: hidden;
    min-width: 300px;
  }

  .gui-panel {
    width: 600px;
    min-width: 400px;
    overflow: hidden;
    border-left: 1px solid var(--border);
  }

  .console-panel {
    border-top: 1px solid var(--border);
    height: 200px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .console-panel.collapsed {
    height: 40px;
  }

  @media (max-width: 1200px) {
    .sidebar {
      width: 220px;
    }
    .gui-panel {
      width: 500px;
    }
  }

  @media (max-width: 900px) {
    .editor-row {
      flex-direction: column;
    }
    .sidebar {
      display: none;
    }
    .editor-panel {
      border-right: none;
      border-bottom: 1px solid var(--border);
      flex: 1;
    }
    .gui-panel {
      width: 100%;
      height: 200px;
      border-left: none;
    }
    .toolbar {
      flex-direction: column;
      align-items: flex-start;
      gap: 10px;
    }
  }
</style>
