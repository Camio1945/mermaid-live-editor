import { listen } from '@tauri-apps/api/event';

/**
 * Check if the app is running inside Tauri.
 * Returns true only when the Tauri invoke API is available.
 */
export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
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

  const unlisten = listen<{ path: string; content: string }>('file-opened', (event) => {
    console.log('[Tauri] File opened:', event.payload.path);
    onFileOpen(event.payload.path, event.payload.content);
  });

  // Return a cleanup function
  let cancelled = false;
  const cleanup = () => {
    if (!cancelled) {
      cancelled = true;
      void unlisten.then((fn) => fn());
    }
  };
  return cleanup;
};
