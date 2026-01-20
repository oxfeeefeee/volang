// VoGUI type definitions

/** Virtual DOM node from Vo */
export interface VoNode {
  type: string;
  props?: Record<string, any>;
  children?: VoNode[];
}

/** Event callback type */
export type EventCallback = (handlerId: number, payload: string) => void;

/** Renderer configuration */
export interface RendererConfig {
  interactive: boolean;
  onEvent?: EventCallback;
}

/** Style property mapping from Vo to CSS */
export const StylePropertyMap: Record<string, string> = {
  padding: 'padding',
  paddingLeft: 'padding-left',
  paddingRight: 'padding-right',
  paddingTop: 'padding-top',
  paddingBottom: 'padding-bottom',
  margin: 'margin',
  marginLeft: 'margin-left',
  marginRight: 'margin-right',
  marginTop: 'margin-top',
  marginBottom: 'margin-bottom',
  background: 'background',
  color: 'color',
  width: 'width',
  height: 'height',
  flex: 'flex',
  gap: 'gap',
  borderRadius: 'border-radius',
  border: 'border',
  boxShadow: 'box-shadow',
  opacity: 'opacity',
  fontSize: 'font-size',
  fontWeight: 'font-weight',
  fontStyle: 'font-style',
  textAlign: 'text-align',
  cursor: 'cursor',
  overflow: 'overflow',
};
