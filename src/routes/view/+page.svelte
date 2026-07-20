<script lang="ts">
  import { page } from '$app/stores';
  import { resolve } from '$app/paths';
  import { Button } from '$/components/ui/button';
  import View from '$/components/View.svelte';
  import { PanZoomState } from '$/util/panZoom';
  import { serializeState } from '$/util/serde';
  import { defaultState } from '$/util/state.svelte';
  import { isTauri, listMmdFiles, revealInExplorer, type MmdFileEntry } from '$/util/tauri';
  import { initHandler } from '$/util/util';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import ArrowBackIcon from '~icons/material-symbols/arrow-back-rounded';
  import ArrowsToCircleIcon from '~icons/material-symbols/screenshot-frame-2';

  // Remember the history length when this page first loads, so we can
  // always go back to the editor regardless of how many files were clicked.
  let initialHistoryLength = $state(0);

  onMount(() => {
    initHandler();
    initialHistoryLength = history.length;
  });

  const goBack = () => {
    // Go back to the editor: jump to the history entry that was current
    // when this view page first loaded (i.e. before any file clicks).
    const stepsBack = history.length - initialHistoryLength + 1;
    history.go(-stepsBack);
  };

  const panZoomState = new PanZoomState();

  // File path passed via ?file= query parameter
  let filePath = $state<string>('');
  let siblingFiles = $state<MmdFileEntry[]>([]);
  let currentFileName = $state('');
  let loadingFile = $state(false);

  $effect(() => {
    const params = $page.url.searchParams;
    const file = params.get('file');
    if (file && isTauri()) {
      filePath = decodeURIComponent(file);
      currentFileName = filePath.replace(/\\/g, '/').split('/').pop() || '';
      loadSiblingFiles(filePath);
    }
  });

  async function loadSiblingFiles(path: string) {
    const files = await listMmdFiles(path);
    siblingFiles = files;
  }

  function findCommonPrefix(names: string[]): string {
    if (names.length <= 1) return '';
    let prefix = names[0];
    for (let i = 1; i < names.length; i++) {
      while (!names[i].startsWith(prefix) && prefix.length > 0) {
        prefix = prefix.slice(0, -1);
      }
    }
    // Trim trailing non-word characters for cleaner display
    prefix = prefix.replace(/[-_~.]+$/, '');
    return prefix;
  }

  function removePrefix(stem: string, prefix: string): string {
    if (!prefix) return stem;
    let result = stem.slice(prefix.length);
    // Clean up leading separators
    result = result.replace(/^[-_~.]+/, '');
    return result || stem;
  }

  let commonPrefix = $derived(findCommonPrefix(siblingFiles.map((f) => f.stem)));

  let prefixDisplay = $derived(
    commonPrefix ? commonPrefix.replace(/[-_~]+/g, (m) => (m === '_' ? '_' : m)) : ''
  );

  let treeItems = $derived(
    siblingFiles.map((f) => ({
      ...f,
      display: commonPrefix ? removePrefix(f.stem, commonPrefix) : f.stem
    }))
  );

  async function openFile(entry: MmdFileEntry) {
    if (loadingFile) return;
    loadingFile = true;
    try {
      const content = await invoke<string>('read_mmd_file', { path: entry.path });
      const serialized = serializeState({
        ...defaultState,
        code: content,
        pan: undefined,
        zoom: undefined
      });
      const fileParam = encodeURIComponent(entry.path);
      // Reset the view before navigating so the new diagram starts fresh
      panZoomState.reset();
      window.location.href = `${resolve('/view', {})}?file=${fileParam}#${serialized}`;
    } catch (err) {
      console.error('Error opening file:', err);
    } finally {
      loadingFile = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'F12' && isTauri() && filePath) {
      e.preventDefault();
      void revealInExplorer(filePath);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <meta name="robots" content="noindex" />
</svelte:head>

<div class="flex h-full">
  <!-- Left sidebar: file list -->
  {#if isTauri() && siblingFiles.length > 0}
    <div class="flex h-full w-52 shrink-0 flex-col border-r border-border bg-muted/30">
      <!-- Current file name header -->
      <div class="shrink-0 border-b border-border px-3 py-2.5">
        <p class="truncate text-xs font-medium text-foreground" title={currentFileName}>
          {currentFileName}
        </p>
      </div>

      <!-- Prefix header -->
      {#if prefixDisplay}
        <div class="shrink-0 px-3 pt-3 pb-1">
          <p
            class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide truncate"
            title={prefixDisplay}>
            {prefixDisplay}
          </p>
        </div>
      {/if}

      <!-- File tree list -->
      <div class="flex-1 overflow-y-auto px-1.5 py-1">
        {#each treeItems as item (item.path)}
          <button
            class="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-xs transition-colors hover:bg-accent hover:text-accent-foreground"
            class:bg-accent={item.path === filePath}
            class:text-accent-foreground={item.path === filePath}
            class:text-muted-foreground={item.path !== filePath}
            onclick={() => openFile(item)}
            title={item.name}>
            <span class="shrink-0 text-muted-foreground/60">
              {#if item.path === filePath}
                <span class="inline-block size-1.5 rounded-full bg-foreground/70"></span>
              {:else}
                <span class="inline-block size-1.5 rounded-full border border-muted-foreground/40"
                ></span>
              {/if}
            </span>
            <span class="truncate">{item.display}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Right: diagram view -->
  <div class="relative flex-1">
    <View {panZoomState} shouldShowGrid={false} />
    <div class="absolute top-4 left-4 z-10 flex flex-col items-center gap-2">
      <Button
        variant="ghost"
        size="icon"
        title="Back to editor"
        class="bg-background/80 backdrop-blur-sm"
        onclick={goBack}>
        <ArrowBackIcon class="size-5" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        title="Reset view"
        class="bg-background/80 backdrop-blur-sm"
        onclick={() => panZoomState.reset()}>
        <ArrowsToCircleIcon />
      </Button>
    </div>
  </div>
</div>
