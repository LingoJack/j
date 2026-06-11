export type BlockType =
  | 'text'
  | 'heading-1'
  | 'heading-2'
  | 'heading-3'
  | 'code'
  | 'table'
  | 'canvas'
  | 'callout'
  | 'image'
  | 'toggle'
  | 'html-render'
  | 'whiteboard';

export interface CanvasPath {
  points: { x: number; y: number }[];
  color: string;
  width: number;
}

export interface BlockProperties {
  language?: string;
  caption?: string;
  isOpen?: boolean; // toggle state
  tableData?: string[][]; // rows x columns for table type
  drawingPaths?: CanvasPath[]; // canvas drawing vector state
  emoji?: string; // callout block icon
  cssCode?: string; // html-render styling
  jsCode?: string; // html-render behavior script
  imageType?: 'url' | 'base64';
  sandboxTheme?: 'light' | 'dark';
}

export interface Block {
  id: string;
  type: BlockType;
  content: string; // also serves as raw code for html-render or markdown text
  properties?: BlockProperties;
}

export interface Document {
  id: string;
  title: string;
  emoji: string;
  createdAt: string;
  updatedAt: string;
  blocks: Block[];
  isFavorite?: boolean;
}

export interface Backlink {
  sourceId: string;
  sourceTitle: string;
  sourceEmoji: string;
}

export interface GraphNode {
  id: string;
  label: string;
  emoji: string;
  group: 'main' | 'sub';
  radius: number;
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
}

export interface GraphLink {
  source: string;
  target: string;
}

export interface Plugin {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  isEnabled: boolean;
  category: 'Editor' | 'Utility' | 'Theme';
}

export interface SyncConfig {
  isEnabled: boolean;
  serverUrl: string;
  deviceId: string;
  lastSyncedAt?: string;
  syncStatus: 'idle' | 'syncing' | 'error' | 'success';
}

export interface LocalAsset {
  id: string;
  name: string;
  type: string;
  size: string;
  createdAt: string;
  content: string; // base64 or placeholder URL
}
