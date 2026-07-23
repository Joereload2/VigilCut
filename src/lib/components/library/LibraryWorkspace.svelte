<script lang="ts">
  import VisualWorkspace from "$lib/components/visual/VisualWorkspace.svelte";
  import type { VisualsViewId } from "$lib/components/visual/library/libraryTypes";

  let view = $state<VisualsViewId>("library");
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden p-2">
  <VisualWorkspace
    bind:view
    libraryOnly
    onViewChange={(next) => (view = next)}
  />
</div>
