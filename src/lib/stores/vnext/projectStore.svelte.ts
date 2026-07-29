import type { ContentProjectSnapshotV1 } from "$lib/types/vnext/v1";
import * as vnext from "$lib/utils/vnext";
import { vnextUiStore } from "./uiStore.svelte";

class VnextProjectStore {
  projects = $state<ContentProjectSnapshotV1[]>([]);
  current = $state<ContentProjectSnapshotV1 | null>(null);
  loading = $state(false);

  async refreshList() {
    this.loading = true;
    vnextUiStore.setError(null);
    try {
      this.projects = await vnext.listContentProjects(50);
    } catch (e) {
      vnextUiStore.setError(String(e));
      this.projects = [];
    } finally {
      this.loading = false;
    }
  }

  async select(id: string) {
    this.loading = true;
    vnextUiStore.setError(null);
    try {
      this.current = await vnext.getContentProject(id);
      vnextUiStore.setNav("projects");
      vnextUiStore.setSection("summary");
    } catch (e) {
      vnextUiStore.setError(String(e));
      this.current = null;
    } finally {
      this.loading = false;
    }
  }

  async createFromPath(sourceMediaPath: string, title?: string) {
    vnextUiStore.setBusy(true, "Creando proyecto…");
    vnextUiStore.setError(null);
    try {
      const p = await vnext.createContentProject({ sourceMediaPath, title });
      this.current = p;
      await this.refreshList();
      return p;
    } catch (e) {
      vnextUiStore.setError(String(e));
      throw e;
    } finally {
      vnextUiStore.setBusy(false);
    }
  }

  clearCurrent() {
    this.current = null;
  }
}

export const vnextProjectStore = new VnextProjectStore();
