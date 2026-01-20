// Vo WASM runtime wrapper

export type RunStatus = 'idle' | 'running' | 'success' | 'error';

export interface RunResult {
  status: 'ok' | 'error' | 'compile_error';
  stdout: string;
  stderr: string;
}

export interface GuiResult {
  status: 'ok' | 'error' | 'compile_error';
  renderJson: string;
  error: string;
}

// WASM module instances
let voWebModule: any = null;
let voguiModule: any = null;

async function loadVoWeb(): Promise<any> {
  if (voWebModule) return voWebModule;

  try {
    const { default: init, compileAndRun, version } = await import('@vo-web/vo_web.js');
    await init();
    voWebModule = { compileAndRun, version };
    console.log('Vo WASM loaded:', version());
    return voWebModule;
  } catch (e) {
    console.error('Failed to load Vo WASM:', e);
    throw new Error('Failed to load Vo runtime. Please refresh the page.');
  }
}

async function loadVogui(): Promise<any> {
  if (voguiModule) return voguiModule;

  try {
    const { default: init, initGuiApp, handleGuiEvent } = await import('@vogui/vogui.js');
    await init();
    voguiModule = { initGuiApp, handleGuiEvent };
    console.log('VoGUI WASM loaded');
    return voguiModule;
  } catch (e) {
    console.error('Failed to load VoGUI WASM:', e);
    throw new Error('Failed to load VoGUI runtime. Please refresh the page.');
  }
}

export async function runCode(source: string): Promise<RunResult> {
  const wasm = await loadVoWeb();

  const result = wasm.compileAndRun(source, 'main.vo');

  return {
    status: result.status,
    stdout: result.stdout || '',
    stderr: result.stderr || '',
  };
}

export async function getVersion(): Promise<string> {
  const wasm = await loadVoWeb();
  return wasm.version();
}

// ============ GUI API ============

export async function initGuiApp(source: string): Promise<GuiResult> {
  const wasm = await loadVogui();
  const result = wasm.initGuiApp(source, 'main.vo');
  return {
    status: result.status,
    renderJson: result.renderJson || '',
    error: result.error || '',
  };
}

export async function handleGuiEvent(handlerId: number, payload: string): Promise<GuiResult> {
  const wasm = await loadVogui();
  const result = wasm.handleGuiEvent(handlerId, payload);
  return {
    status: result.status,
    renderJson: result.renderJson || '',
    error: result.error || '',
  };
}
