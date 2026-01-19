<script lang="ts">
  import type { RunStatus } from '../wasm/vo.ts';

  let { stdout, stderr, status }: { stdout: string; stderr: string; status: RunStatus } = $props();

  const statusLabels: Record<RunStatus, string> = {
    idle: 'Ready',
    running: 'Running...',
    success: 'Success',
    error: 'Error',
  };
</script>

<div class="output">
  <div class="output-header">
    <span class="output-title">Output</span>
    <span class="status" class:success={status === 'success'} class:error={status === 'error'}>
      {statusLabels[status]}
    </span>
  </div>
  <div class="output-content">
    {#if stdout}
      <pre class="stdout">{stdout}</pre>
    {/if}
    {#if stderr}
      <pre class="stderr">{stderr}</pre>
    {/if}
    {#if !stdout && !stderr && status === 'idle'}
      <p class="placeholder">Click "Run" to execute your code</p>
    {/if}
    {#if !stdout && !stderr && status === 'success'}
      <p class="placeholder">Program completed with no output</p>
    {/if}
  </div>
</div>

<style>
  .output {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .output-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .output-title {
    font-weight: 600;
    font-size: 14px;
  }

  .status {
    font-size: 12px;
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .status.success {
    background: rgba(78, 201, 176, 0.2);
    color: var(--success);
  }

  .status.error {
    background: rgba(241, 76, 76, 0.2);
    color: var(--error);
  }

  .output-content {
    flex: 1;
    padding: 16px;
    overflow: auto;
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 13px;
    line-height: 1.5;
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .stdout {
    color: var(--text-primary);
  }

  .stderr {
    color: var(--error);
    margin-top: 8px;
  }

  .placeholder {
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
