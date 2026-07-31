<script lang="ts">
  import type { ClipCandidate } from "$lib/types";
  import ShortPreview from "$lib/vnext/components/ShortPreview.svelte";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";

  interface Props {
    candidate: ClipCandidate;
    sourcePath: string | null;
  }

  let { candidate, sourcePath }: Props = $props();
</script>

<div class="flex w-full flex-col items-center gap-3" data-testid="candidate-preview">
  <ShortPreview
    sourcePath={sourcePath || candidate.sourceMediaPath}
    start={candidate.start}
    end={candidate.end}
    centerX={candidate.framing.centerX}
    centerY={candidate.framing.centerY}
    autoplay={true}
    label="{formatDuration(candidate.start)} – {formatDuration(candidate.end)} en el video original"
  />
  <div class="w-full max-w-md">
    <div class="h-1.5 overflow-hidden rounded-full bg-surface-800" data-testid="mini-timeline">
      <div
        class="h-full rounded-full bg-amber-500/80"
        style="margin-left: {Math.min(90, (candidate.start / Math.max(candidate.end, 1)) * 10)}%; width: {Math.min(100, (candidate.duration / 60) * 100)}%"
      ></div>
    </div>
    <p class="mt-2 text-center text-sm text-surface-300">
      {candidate.summary || candidate.transcript?.slice(0, 160)}
    </p>
  </div>
</div>
