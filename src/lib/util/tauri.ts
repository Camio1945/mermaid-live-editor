import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

/**
 * Check if the app is running inside Tauri.
 * Returns true only when the Tauri invoke API is available.
 */
export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

/**
 * Payload emitted by the Rust backend when a .mmd file is opened
 * (via command-line, file-association double-click, or drag-and-drop).
 */
export interface FileOpenedPayload {
  path: string;
  content: string;
}

/**
 * Show the native file-open dialog filtered to mermaid source files and
 * invoke the Rust `read_mmd_file` command to read the selected file.
 * Returns `null` if the user cancels the dialog.
 */
export const pickMermaidFile = async (): Promise<FileOpenedPayload | null> => {
  if (!isTauri()) {
    return null;
  }
  const selected = await openDialog({
    multiple: false,
    directory: false,
    title: 'Open Mermaid File',
    filters: [
      { name: 'Mermaid Diagram', extensions: ['mmd', 'mermaid'] },
      { name: 'All Files', extensions: ['*'] }
    ]
  });
  if (!selected || Array.isArray(selected)) {
    return null;
  }
  const content = await invoke<string>('read_mmd_file', { path: selected });
  return { path: selected, content };
};

/**
 * Setup listener for file-open events from the Tauri backend.
 * When a .mmd file is double-clicked or dragged onto the app,
 * the backend emits a 'file-opened' event with the file path and content.
 */
export const listenForFileOpen = (
  onFileOpen: (path: string, content: string) => void
): (() => void) => {
  if (!isTauri()) {
    return () => {};
  }

  const unlistenPromise: Promise<UnlistenFn> = listen<FileOpenedPayload>('file-opened', (event) => {
    console.log('[Tauri] File opened:', event.payload.path);
    onFileOpen(event.payload.path, event.payload.content);
  });

  // Return a cleanup function
  let cancelled = false;
  const cleanup = () => {
    if (!cancelled) {
      cancelled = true;
      void unlistenPromise.then((fn) => fn());
    }
  };
  return cleanup;
};
