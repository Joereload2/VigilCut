/**
 * Job snapshot slice for vNext MVP.
 * Presentation labels must go through jobLabels helpers — never raw enums in UI.
 */
import type { JobSnapshotV1 } from "$lib/types/vnext/v1";

class JobStore {
  items = $state<JobSnapshotV1[]>([]);

  setItems(items: JobSnapshotV1[]) {
    this.items = items;
  }

  get activeRender(): JobSnapshotV1 | null {
    return (
      this.items.find(
        (j) =>
          j.kind === "vertical_render" &&
          (j.status === "queued" || j.status === "running" || j.status === "cancelling"),
      ) ?? null
    );
  }

  clear() {
    this.items = [];
  }
}

export const jobStore = new JobStore();
