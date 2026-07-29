import type { ArtifactManifestV1, JobSnapshotV1 } from "$lib/types/vnext/v1";
import * as vnext from "$lib/utils/vnext";
import { vnextUiStore } from "./uiStore.svelte";

class VnextJobStore {
  projectJobs = $state<JobSnapshotV1[]>([]);
  queue = $state<JobSnapshotV1[]>([]);
  artifacts = $state<ArtifactManifestV1[]>([]);
  lastJob = $state<JobSnapshotV1 | null>(null);

  async refreshProject(contentProjectId: string) {
    try {
      this.projectJobs = await vnext.listProjectJobs(contentProjectId);
      this.artifacts = await vnext.listProjectArtifacts(contentProjectId);
    } catch (e) {
      vnextUiStore.setError(String(e));
    }
  }

  async refreshQueue() {
    try {
      this.queue = await vnext.listQueueJobs(100);
    } catch (e) {
      vnextUiStore.setError(String(e));
      this.queue = [];
    }
  }

  async retry(id: string) {
    vnextUiStore.setBusy(true, "Reintentando…");
    try {
      this.lastJob = await vnext.retryJob(id);
      await this.refreshQueue();
      if (this.lastJob.contentProjectId) {
        await this.refreshProject(this.lastJob.contentProjectId);
      }
    } catch (e) {
      vnextUiStore.setError(String(e));
    } finally {
      vnextUiStore.setBusy(false);
    }
  }

  async cancel(id: string) {
    vnextUiStore.setBusy(true, "Cancelando…");
    try {
      this.lastJob = await vnext.cancelJob(id);
      await this.refreshQueue();
    } catch (e) {
      vnextUiStore.setError(String(e));
    } finally {
      vnextUiStore.setBusy(false);
    }
  }

  async startRender(planId: string) {
    vnextUiStore.setBusy(true, "Render vertical…");
    try {
      this.lastJob = await vnext.startVerticalRender(planId);
      if (this.lastJob.contentProjectId) {
        await this.refreshProject(this.lastJob.contentProjectId);
      }
      await this.refreshQueue();
      return this.lastJob;
    } catch (e) {
      vnextUiStore.setError(String(e));
      throw e;
    } finally {
      vnextUiStore.setBusy(false);
    }
  }
}

export const vnextJobStore = new VnextJobStore();
