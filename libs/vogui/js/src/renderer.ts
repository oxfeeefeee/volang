// VoGUI DOM Renderer
// Framework-agnostic rendering of VoNode tree to DOM

import { VoNode, RendererConfig, StylePropertyMap } from './types';

/** Convert Vo style object to CSS string */
export function styleToString(style: Record<string, any> | undefined): string {
  if (!style || typeof style !== 'object') return '';
  
  return Object.entries(style)
    .map(([key, value]) => {
      const cssProp = StylePropertyMap[key] || key;
      const cssValue = typeof value === 'number' ? `${value}px` : value;
      return `${cssProp}: ${cssValue}`;
    })
    .join('; ');
}

/** Create event handlers bound to config */
function createEventHandlers(config: RendererConfig) {
  const { interactive, onEvent } = config;
  
  return {
    handleClick(handlerId: number | undefined) {
      if (interactive && onEvent && handlerId !== undefined) {
        onEvent(handlerId, '{}');
      }
    },
    
    handleInput(handlerId: number | undefined, value: string) {
      if (interactive && onEvent && handlerId !== undefined) {
        onEvent(handlerId, JSON.stringify({ value }));
      }
    },
    
    handleChecked(handlerId: number | undefined, checked: boolean) {
      if (interactive && onEvent && handlerId !== undefined) {
        onEvent(handlerId, JSON.stringify({ checked }));
      }
    },
    
    handleSlider(handlerId: number | undefined, value: number) {
      if (interactive && onEvent && handlerId !== undefined) {
        onEvent(handlerId, JSON.stringify({ value }));
      }
    },
  };
}

/** Render a VoNode tree to DOM element */
export function renderNode(node: VoNode, config: RendererConfig, handlers?: ReturnType<typeof createEventHandlers>): HTMLElement | Text | null {
  const { interactive } = config;
  if (!handlers) handlers = createEventHandlers(config);
  const style = styleToString(node.props?.style);
  
  function renderChildren(parent: HTMLElement, children?: VoNode[]) {
    if (!children) return;
    for (const child of children) {
      const el = renderNode(child, config, handlers);
      if (el) parent.appendChild(el);
    }
  }
  
  switch (node.type) {
    // Layout
    case 'Column': {
      const el = document.createElement('div');
      el.className = 'vo-column';
      if (style) el.style.cssText = style;
      renderChildren(el, node.children);
      return el;
    }
    
    case 'Row': {
      const el = document.createElement('div');
      el.className = 'vo-row';
      if (style) el.style.cssText = style;
      renderChildren(el, node.children);
      return el;
    }
    
    case 'Center': {
      const el = document.createElement('div');
      el.className = 'vo-center';
      if (style) el.style.cssText = style;
      if (node.children?.[0]) {
        const child = renderNode(node.children[0], config);
        if (child) el.appendChild(child);
      }
      return el;
    }
    
    case 'Wrap': {
      const el = document.createElement('div');
      el.className = 'vo-wrap';
      if (style) el.style.cssText = style;
      renderChildren(el, node.children);
      return el;
    }
    
    case 'Grid': {
      const el = document.createElement('div');
      el.className = 'vo-grid';
      const cols = node.props?.cols ?? 2;
      el.style.cssText = `grid-template-columns: repeat(${cols}, 1fr); ${style}`;
      renderChildren(el, node.children);
      return el;
    }
    
    case 'Scroll': {
      const el = document.createElement('div');
      el.className = 'vo-scroll';
      if (style) el.style.cssText = style;
      if (node.children?.[0]) {
        const child = renderNode(node.children[0], config);
        if (child) el.appendChild(child);
      }
      return el;
    }
    
    case 'Fragment': {
      const el = document.createElement('div');
      el.style.display = 'contents';
      renderChildren(el, node.children);
      return el;
    }
    
    case 'Show': {
      const el = document.createElement('div');
      el.className = 'vo-show';
      el.style.cssText = `display: ${node.props?.visible ? 'contents' : 'none'}; ${style}`;
      if (node.children?.[0]) {
        const child = renderNode(node.children[0], config);
        if (child) el.appendChild(child);
      }
      return el;
    }
    
    case 'Block': {
      const el = document.createElement('div');
      el.className = 'vo-block';
      if (style) el.style.cssText = style;
      renderChildren(el, node.children);
      return el;
    }
    
    // Text
    case 'Text': {
      const el = document.createElement('span');
      el.className = 'vo-text';
      if (style) el.style.cssText = style;
      el.textContent = node.props?.content ?? '';
      return el;
    }
    
    case 'H1': {
      const el = document.createElement('h1');
      el.className = 'vo-h1';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'H2': {
      const el = document.createElement('h2');
      el.className = 'vo-h2';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'H3': {
      const el = document.createElement('h3');
      el.className = 'vo-h3';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'H4': {
      const el = document.createElement('h4');
      el.className = 'vo-h4';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'H5': {
      const el = document.createElement('h5');
      el.className = 'vo-h5';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'H6': {
      const el = document.createElement('h6');
      el.className = 'vo-h6';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'P': {
      const el = document.createElement('p');
      el.className = 'vo-p';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'Code': {
      const el = document.createElement('code');
      el.className = 'vo-code';
      el.textContent = node.props?.code ?? '';
      return el;
    }
    
    case 'Pre': {
      const el = document.createElement('pre');
      el.className = 'vo-pre';
      el.textContent = node.props?.code ?? '';
      return el;
    }
    
    case 'Strong': {
      const el = document.createElement('strong');
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'Em': {
      const el = document.createElement('em');
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'Link': {
      const el = document.createElement('a');
      el.className = 'vo-link';
      el.href = node.props?.href ?? '#';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    // Display
    case 'Badge': {
      const el = document.createElement('span');
      el.className = 'vo-badge';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'Tag': {
      const el = document.createElement('span');
      el.className = 'vo-tag';
      el.textContent = node.props?.text ?? '';
      return el;
    }
    
    case 'Progress': {
      const el = document.createElement('div');
      el.className = 'vo-progress';
      const bar = document.createElement('div');
      bar.className = 'vo-progress-bar';
      bar.style.width = `${(node.props?.value ?? 0) * 100}%`;
      el.appendChild(bar);
      return el;
    }
    
    case 'Spinner': {
      const el = document.createElement('div');
      el.className = 'vo-spinner';
      return el;
    }
    
    case 'Alert': {
      const el = document.createElement('div');
      el.className = `vo-alert vo-alert-${node.props?.kind ?? 'info'}`;
      el.textContent = node.props?.message ?? '';
      return el;
    }
    
    case 'Image': {
      const el = document.createElement('img');
      el.className = 'vo-image';
      el.src = node.props?.src ?? '';
      el.alt = '';
      return el;
    }
    
    case 'Icon': {
      const el = document.createElement('span');
      el.className = 'vo-icon';
      el.textContent = node.props?.name ?? '?';
      return el;
    }
    
    case 'Divider': {
      const el = document.createElement('hr');
      el.className = 'vo-divider';
      return el;
    }
    
    // Interactive
    case 'Button': {
      const el = document.createElement('button');
      el.className = 'vo-button' + (interactive ? ' interactive' : '');
      el.disabled = !interactive;
      el.textContent = node.props?.text ?? 'Button';
      if (style) el.style.cssText = style;
      el.onclick = () => handlers.handleClick(node.props?.onClick);
      return el;
    }
    
    case 'IconButton': {
      const el = document.createElement('button');
      el.className = 'vo-icon-button' + (interactive ? ' interactive' : '');
      el.disabled = !interactive;
      el.textContent = node.props?.icon ?? '?';
      el.onclick = () => handlers.handleClick(node.props?.onClick);
      return el;
    }
    
    case 'Input': {
      const el = document.createElement('input');
      el.className = 'vo-input';
      el.type = 'text';
      el.value = node.props?.value ?? '';
      el.disabled = !interactive;
      if (style) el.style.cssText = style;
      el.oninput = () => handlers.handleInput(node.props?.onChange, el.value);
      return el;
    }
    
    case 'Password': {
      const el = document.createElement('input');
      el.className = 'vo-input';
      el.type = 'password';
      el.value = node.props?.value ?? '';
      el.disabled = !interactive;
      el.oninput = () => handlers.handleInput(node.props?.onChange, el.value);
      return el;
    }
    
    case 'TextArea': {
      const el = document.createElement('textarea');
      el.className = 'vo-textarea';
      el.value = node.props?.value ?? '';
      el.disabled = !interactive;
      el.oninput = () => handlers.handleInput(node.props?.onChange, el.value);
      return el;
    }
    
    case 'Checkbox': {
      const label = document.createElement('label');
      label.className = 'vo-checkbox';
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.checked = node.props?.checked ?? false;
      input.disabled = !interactive;
      input.onchange = () => handlers.handleChecked(node.props?.onChange, input.checked);
      label.appendChild(input);
      return label;
    }
    
    case 'CheckboxLabel': {
      const label = document.createElement('label');
      label.className = 'vo-checkbox-label';
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.checked = node.props?.checked ?? false;
      input.disabled = !interactive;
      input.onchange = () => handlers.handleChecked(node.props?.onChange, input.checked);
      const span = document.createElement('span');
      span.textContent = node.props?.label ?? '';
      label.appendChild(input);
      label.appendChild(span);
      return label;
    }
    
    case 'Switch': {
      const label = document.createElement('label');
      label.className = 'vo-switch';
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.checked = node.props?.on ?? false;
      input.disabled = !interactive;
      input.onchange = () => handlers.handleChecked(node.props?.onChange, input.checked);
      const slider = document.createElement('span');
      slider.className = 'vo-switch-slider';
      label.appendChild(input);
      label.appendChild(slider);
      return label;
    }
    
    case 'Select': {
      const el = document.createElement('select');
      el.className = 'vo-select';
      el.disabled = !interactive;
      for (const opt of (node.props?.options ?? [])) {
        const option = document.createElement('option');
        option.value = opt;
        option.textContent = opt;
        option.selected = opt === node.props?.value;
        el.appendChild(option);
      }
      el.onchange = () => handlers.handleInput(node.props?.onChange, el.value);
      return el;
    }
    
    case 'Slider': {
      const el = document.createElement('input');
      el.className = 'vo-slider';
      el.type = 'range';
      el.min = String(node.props?.min ?? 0);
      el.max = String(node.props?.max ?? 100);
      el.value = String(node.props?.value ?? 0);
      el.disabled = !interactive;
      el.oninput = () => handlers.handleSlider(node.props?.onChange, parseInt(el.value));
      return el;
    }
    
    case 'NumberInput': {
      const el = document.createElement('input');
      el.className = 'vo-input vo-number-input';
      el.type = 'number';
      el.value = String(node.props?.value ?? 0);
      el.disabled = !interactive;
      el.oninput = () => handlers.handleSlider(node.props?.onChange, parseInt(el.value) || 0);
      return el;
    }
    
    // Utility
    case 'Spacer': {
      const el = document.createElement('div');
      el.className = 'vo-spacer';
      return el;
    }
    
    case 'Empty':
      return null;
    
    default: {
      const el = document.createElement('div');
      el.className = 'vo-unknown';
      el.textContent = `[${node.type}]`;
      return el;
    }
  }
}

/** Render VoNode tree into a container element */
export function render(container: HTMLElement, tree: VoNode | null, config: RendererConfig): void {
  container.innerHTML = '';
  if (tree) {
    const el = renderNode(tree, config);
    if (el) container.appendChild(el);
  }
}

/** Setup global key handler */
export function setupKeyHandler(config: RendererConfig): () => void {
  const handler = (event: KeyboardEvent) => {
    if (config.interactive && config.onEvent) {
      config.onEvent(-2, JSON.stringify({ key: event.key }));
    }
  };
  
  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}
