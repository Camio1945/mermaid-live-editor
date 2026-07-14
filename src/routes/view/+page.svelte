<script lang="ts">
  import { Button } from '$/components/ui/button';
  import View from '$/components/View.svelte';
  import { initHandler } from '$/util/util';
  import { onMount } from 'svelte';
  import ArrowBackIcon from '~icons/material-symbols/arrow-back-rounded';

  onMount(initHandler);

  const goBack = () => {
    history.back();
  };
</script>

<svelte:head>
  <meta name="robots" content="noindex" />
</svelte:head>

<div class="relative h-full">
  <View shouldShowGrid={false} />
  <Button
    variant="ghost"
    size="icon"
    title="Back to editor"
    class="absolute top-4 left-4 z-10 bg-background/80 backdrop-blur-sm"
    onclick={goBack}>
    <ArrowBackIcon class="size-5" />
  </Button>
</div>
