<script lang="ts">
  import { examples } from '../examples/index.ts';

  let { onSelect }: { onSelect: (example: { name: string; code: string }) => void } = $props();
</script>

<div class="examples">
  <span class="label">Examples:</span>
  <div class="example-list">
    {#each examples as example}
      <button class="example-btn" onclick={() => onSelect(example)}>
        {example.name}
      </button>
    {/each}
  </div>
</div>

<style>
  .examples {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .label {
    font-size: 13px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .example-list {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .example-btn {
    padding: 4px 10px;
    font-size: 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .example-btn:hover {
    background: var(--border);
    border-color: var(--accent);
  }
</style>
