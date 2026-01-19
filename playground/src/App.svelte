<script lang="ts">
  import './app.css';
  import Editor from './components/Editor.svelte';
  import Output from './components/Output.svelte';
  import Examples from './components/Examples.svelte';
  import { runCode, type RunStatus } from './wasm/vo.ts';

  let code = $state(`package main

func main() {
    println("Hello, Vo!")
    
    for i := 0; i < 5; i++ {
        println("Count:", i)
    }
}
`);

  let stdout = $state('');
  let stderr = $state('');
  let status: RunStatus = $state('idle');

  async function handleRun() {
    status = 'running';
    stdout = '';
    stderr = '';

    try {
      const result = await runCode(code);
      stdout = result.stdout;
      stderr = result.stderr;
      status = result.status === 'ok' ? 'success' : 'error';
    } catch (e) {
      stderr = e instanceof Error ? e.message : String(e);
      status = 'error';
    }
  }

  function handleReset() {
    stdout = '';
    stderr = '';
    status = 'idle';
  }

  function handleExampleSelect(example: { code: string }) {
    code = example.code;
    handleReset();
  }
</script>

<div class="playground">
  <header class="header">
    <div class="logo">
      <span class="logo-text">Vo Playground</span>
    </div>
    <div class="actions">
      <button class="btn-primary" onclick={handleRun} disabled={status === 'running'}>
        {status === 'running' ? 'Running...' : 'Run'}
      </button>
      <button class="btn-secondary" onclick={handleReset}>
        Reset
      </button>
    </div>
  </header>

  <main class="main">
    <div class="editor-panel">
      <Editor bind:value={code} />
    </div>
    <div class="output-panel">
      <Output {stdout} {stderr} {status} />
    </div>
  </main>

  <footer class="footer">
    <Examples onSelect={handleExampleSelect} />
  </footer>
</div>

<style>
  .playground {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .logo-text {
    font-size: 18px;
    font-weight: 600;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .main {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .editor-panel {
    flex: 1;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .output-panel {
    width: 400px;
    min-width: 300px;
    overflow: hidden;
  }

  .footer {
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    padding: 8px 20px;
  }

  @media (max-width: 900px) {
    .main {
      flex-direction: column;
    }
    .editor-panel {
      border-right: none;
      border-bottom: 1px solid var(--border);
      height: 50%;
    }
    .output-panel {
      width: 100%;
      height: 50%;
    }
  }
</style>
