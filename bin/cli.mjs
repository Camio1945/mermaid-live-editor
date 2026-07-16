#!/usr/bin/env node

/**
 * Mermaid Live Editor CLI — Syntax validator.
 *
 * Usage:
 *   node bin/cli.mjs validate <file.mmd>     Validate syntax
 *   node bin/cli.mjs check <file.mmd>        Alias for validate
 *   node bin/cli.mjs -v <file.mmd>           Alias for validate
 *   node bin/cli.mjs -c <file.mmd>           Alias for validate
 *   node bin/cli.mjs --help, -h              Show this help
 *
 * Output:
 *   Valid syntax: prints nothing (empty), exit code 0
 *   Syntax error: prints the error message to stdout, exit code 1
 *   File error: prints error to stdout, exit code 2
 */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseHTML } from 'linkedom';

// ── Set up DOM environment for mermaid/dompurify ──────────────────────────────

const { document, window: win } = parseHTML('<!DOCTYPE html><html><body></body></html>');

globalThis.window = win;
globalThis.document = document;
globalThis.Node = win.Node;
globalThis.Element = win.Element;
globalThis.DOMParser = win.DOMParser;
globalThis.XMLSerializer = win.XMLSerializer;
globalThis.DocumentFragment = win.DocumentFragment;
globalThis.HTMLTemplateElement = win.HTMLTemplateElement;
globalThis.NodeFilter = win.NodeFilter;
globalThis.NamedNodeMap = win.NamedNodeMap;
globalThis.HTMLCollection = win.HTMLCollection;
globalThis.NodeList = win.NodeList;

// ── Help ──────────────────────────────────────────────────────────────────────

function printHelp() {
  console.log(`Mermaid Live Editor CLI

USAGE:
  mermaid-live-editor validate <file>    Validate mermaid diagram syntax
  mermaid-live-editor check <file>       Alias for validate
  mermaid-live-editor -v <file>          Alias for validate
  mermaid-live-editor -c <file>          Alias for validate
  mermaid-live-editor --help, -h         Show this help

EXAMPLES:
  mermaid-live-editor validate diagram.mmd
  mermaid-live-editor check diagram.mmd
  mermaid-live-editor -v diagram.mmd
  mermaid-live-editor -c diagram.mmd
  mermaid-live-editor -h

DESCRIPTION:
  Reads a .mmd or .mermaid file and checks its mermaid syntax.
  On success: prints nothing (empty output).
  On error: prints the error message.

  Exit codes:
    0 — syntax is valid
    1 — syntax error detected
    2 — file not found or read error
    3 — unknown command`);
}

// ── Mermaid resolution ────────────────────────────────────────────────────────

let _mermaidModule = null;

async function getMermaid() {
  if (_mermaidModule) return _mermaidModule;

  // Try direct import (works when bundled with esbuild)
  try {
    _mermaidModule = await import('mermaid');
    return _mermaidModule;
  } catch {
    // Not bundled — try project node_modules
  }

  // Dev mode: resolve from project node_modules
  const { fileURLToPath } = await import('node:url');
  const { dirname } = await import('node:path');
  const { createRequire } = await import('node:module');

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const require = createRequire(import.meta.url);

  try {
    const mermaidPath = resolve(__dirname, '..', 'node_modules', 'mermaid');
    _mermaidModule = await import(mermaidPath);
    return _mermaidModule;
  } catch {
    try {
      const resolvedPath = require.resolve('mermaid');
      _mermaidModule = await import(resolvedPath);
      return _mermaidModule;
    } catch {
      throw new Error(
        'Cannot find mermaid package. Make sure mermaid is installed (pnpm install).'
      );
    }
  }
}

// ── Validate command ───────────────────────────────────────────────────────────

async function validateCommand(filePath) {
  // Read the file
  let code;
  try {
    code = readFileSync(resolve(filePath), 'utf-8');
  } catch (err) {
    console.log(`Failed to read file: ${err.message}`);
    process.exit(2);
  }

  // Basic pre-check: empty file
  if (code.trim().length === 0) {
    console.log('File is empty. No mermaid diagram code found.');
    process.exit(1);
  }

  // Load mermaid
  let mermaid;
  try {
    mermaid = await getMermaid();
  } catch (err) {
    console.log(`Failed to load mermaid: ${err.message}`);
    process.exit(1);
  }

  // Initialize mermaid with proper config.
  // The DOM environment is already set up above via linkedom,
  // so dompurify will work correctly.
  try {
    const cfg = mermaid.default.mermaidAPI?.defaultConfig ?? {};
    mermaid.default.initialize({
      ...cfg,
      securityLevel: cfg.securityLevel || 'strict',
      startOnLoad: false
    });
  } catch {
    // Fallback with loose security
    try {
      mermaid.default.initialize({ securityLevel: 'loose', startOnLoad: false });
    } catch {
      // Last resort
    }
  }

  // Parse the diagram code
  try {
    await mermaid.default.parse(code);
    // Valid syntax — print nothing (caller checks exit code)
    process.exit(0);
  } catch (parseError) {
    // Invalid syntax — print the raw error message
    const message = parseError.message || String(parseError);
    console.log(message);
    process.exit(1);
  }
}

// ── Main ───────────────────────────────────────────────────────────────────────

async function main() {
  const args = process.argv.slice(2);

  if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
    printHelp();
    process.exit(0);
  }

  const command = args[0];

  // Aliases: -v, check, -c all map to validate
  if (command === 'validate' || command === 'check' || command === '-v' || command === '-c') {
    const filePath = args[1];

    if (!filePath) {
      console.log('Error: Missing file path. Usage: mermaid-live-editor validate <file>');
      process.exit(3);
    }

    await validateCommand(filePath);
  } else {
    console.log(`Error: Unknown command "${command}"`);
    console.log('Run "mermaid-live-editor --help" for usage information.');
    process.exit(3);
  }
}

main();
