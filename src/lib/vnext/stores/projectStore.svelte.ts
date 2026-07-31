/**
 * Project snapshot slice for vNext MVP.
 * Canonical project data lives here; stage is never stored — always derived.
 */
import type { ContentProjectSnapshotV1 } from "$lib/types/vnext/v1";

class ProjectStore {
  list = $state<ContentProjectSnapshotV1[]>([]);
  current = $state<ContentProjectSnapshotV1 | null>(null);
  loading = $state(false);

  setList(projects: ContentProjectSnapshotV1[]) {
    this.list = projects;
  }

  setCurrent(project: ContentProjectSnapshotV1 | null) {
    this.current = project;
  }

  clear() {
    this.current = null;
  }
}

export const projectStore = new ProjectStore();
