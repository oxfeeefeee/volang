// VoGUI CSS styles
// These styles can be injected into the document or imported as a stylesheet

export const voguiStyles = `
/* Layout Components */
.vo-column {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.vo-row {
  display: flex;
  flex-direction: row;
  gap: 8px;
  align-items: center;
}

.vo-center {
  display: flex;
  align-items: center;
  justify-content: center;
}

.vo-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.vo-grid {
  display: grid;
  gap: 8px;
}

.vo-scroll {
  overflow: auto;
  max-height: 300px;
}

.vo-block {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
}

.vo-spacer {
  flex: 1;
}

/* Typography */
.vo-text {
  font-size: 14px;
  color: #333;
}

.vo-h1, .vo-h2, .vo-h3, .vo-h4, .vo-h5, .vo-h6 {
  margin: 0;
  color: inherit;
}
.vo-h1 { font-size: 2em; }
.vo-h2 { font-size: 1.5em; }
.vo-h3 { font-size: 1.17em; }
.vo-h4 { font-size: 1em; }
.vo-h5 { font-size: 0.83em; }
.vo-h6 { font-size: 0.67em; }

.vo-p {
  margin: 0;
}

.vo-code {
  font-family: monospace;
  background: #f5f5f5;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 13px;
}

.vo-pre {
  font-family: monospace;
  background: #f5f5f5;
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 0;
  font-size: 13px;
}

.vo-link {
  color: #007bff;
  text-decoration: none;
}
.vo-link:hover {
  text-decoration: underline;
}

/* Badges & Tags */
.vo-badge {
  display: inline-block;
  padding: 2px 8px;
  font-size: 12px;
  font-weight: 600;
  background: #007bff;
  color: white;
  border-radius: 10px;
}

.vo-tag {
  display: inline-block;
  padding: 2px 8px;
  font-size: 12px;
  background: #f0f0f0;
  color: #666;
  border-radius: 4px;
}

/* Progress & Spinner */
.vo-progress {
  height: 8px;
  background: #e0e0e0;
  border-radius: 4px;
  overflow: hidden;
  width: 100%;
}

.vo-progress-bar {
  height: 100%;
  background: #007bff;
  transition: width 0.2s;
}

.vo-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid #e0e0e0;
  border-top-color: #007bff;
  border-radius: 50%;
  animation: vo-spin 0.8s linear infinite;
}

@keyframes vo-spin {
  to { transform: rotate(360deg); }
}

/* Alerts */
.vo-alert {
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 14px;
}
.vo-alert-info { background: #e3f2fd; color: #1565c0; }
.vo-alert-success { background: #e8f5e9; color: #2e7d32; }
.vo-alert-warning { background: #fff3e0; color: #ef6c00; }
.vo-alert-error { background: #ffebee; color: #c62828; }

/* Media */
.vo-image {
  max-width: 100%;
  border-radius: 6px;
}

.vo-icon {
  font-size: 18px;
}

.vo-divider {
  border: none;
  border-top: 1px solid #e0e0e0;
  margin: 8px 0;
}

/* Buttons */
.vo-button {
  padding: 8px 16px;
  font-size: 14px;
  border: none;
  border-radius: 6px;
  background: #007bff;
  color: white;
  cursor: not-allowed;
  opacity: 0.8;
}

.vo-button.interactive {
  cursor: pointer;
  opacity: 1;
}

.vo-button.interactive:hover {
  filter: brightness(1.1);
}

.vo-button.interactive:active {
  filter: brightness(0.95);
}

.vo-icon-button {
  padding: 8px;
  font-size: 16px;
  border: none;
  border-radius: 6px;
  background: #f0f0f0;
  color: inherit;
  cursor: not-allowed;
  opacity: 0.8;
}

.vo-icon-button.interactive {
  cursor: pointer;
  opacity: 1;
}

.vo-icon-button.interactive:hover {
  background: #e0e0e0;
}

/* Form Controls */
.vo-input {
  padding: 8px 12px;
  font-size: 14px;
  border: 1px solid #ccc;
  border-radius: 6px;
  background: white;
  color: inherit;
}

.vo-input:focus {
  outline: none;
  border-color: #007bff;
}

.vo-textarea {
  padding: 8px 12px;
  font-size: 14px;
  border: 1px solid #ccc;
  border-radius: 6px;
  background: white;
  color: inherit;
  min-height: 80px;
  resize: vertical;
  font-family: inherit;
}

.vo-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
}

.vo-checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.vo-switch {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
}

.vo-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.vo-switch-slider {
  position: absolute;
  cursor: pointer;
  inset: 0;
  background: #ccc;
  border-radius: 22px;
  transition: 0.2s;
}

.vo-switch-slider::before {
  content: '';
  position: absolute;
  height: 18px;
  width: 18px;
  left: 2px;
  bottom: 2px;
  background: white;
  border-radius: 50%;
  transition: 0.2s;
}

.vo-switch input:checked + .vo-switch-slider {
  background: #007bff;
}

.vo-switch input:checked + .vo-switch-slider::before {
  transform: translateX(18px);
}

.vo-select {
  padding: 8px 12px;
  font-size: 14px;
  border: 1px solid #ccc;
  border-radius: 6px;
  background: white;
  color: inherit;
  cursor: pointer;
}

.vo-slider {
  width: 100%;
  cursor: pointer;
}

.vo-number-input {
  width: 80px;
}

/* Unknown/Error */
.vo-unknown {
  padding: 4px 8px;
  background: #fee;
  color: #c00;
  border-radius: 4px;
  font-family: monospace;
  font-size: 12px;
}
`;

/** Inject VoGUI styles into document head */
export function injectStyles(): void {
  if (typeof document === 'undefined') return;
  
  const id = 'vogui-styles';
  if (document.getElementById(id)) return;
  
  const style = document.createElement('style');
  style.id = id;
  style.textContent = voguiStyles;
  document.head.appendChild(style);
}
