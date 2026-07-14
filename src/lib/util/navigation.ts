import { isTauri } from './tauri';

/**
 * Check if a URL is internal (same-origin, within the app).
 * Internal URLs are relative paths or absolute URLs on the same origin.
 */
const isInternalUrl = (url: string): boolean => {
  if (url.startsWith('#')) {
    return true;
  }
  if (!url.includes('://')) {
    return true; // relative URL like /edit or /view
  }
  try {
    return new URL(url).origin === window.location.origin;
  } catch {
    return false;
  }
};

/**
 * Open a URL. In Tauri:
 * - Internal URLs navigate within the app window (no new tab/window).
 * - External URLs open in the system browser.
 *
 * In a regular browser, external URLs use window.open (new tab),
 * and internal URLs navigate in the same tab.
 */
export const openUrl = (url: string): void => {
  if (isInternalUrl(url)) {
    window.location.assign(url);
  } else if (isTauri()) {
    // In Tauri, external URLs open in the system browser.
    // The tauri-plugin-opener intercepts window.open with _blank.
    window.open(url, '_blank', 'noopener');
  } else {
    // In a regular browser, open external links in a new tab.
    window.open(url, '_blank', 'noopener');
  }
};
