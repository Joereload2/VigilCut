import type { ProjectSectionV1, VnextNavV1 } from "$lib/types/vnext/v1";

/** Ephemeral UI chrome for vNext shell (not durable). */
class VnextUiStore {
  nav = $state<VnextNavV1>("projects");
  projectSection = $state<ProjectSectionV1>("summary");
  showLegacyTools = $state(false);
  busy = $state(false);
  statusMessage = $state("");
  error = $state<string | null>(null);

  setNav(n: VnextNavV1) {
    this.nav = n;
    if (n === "legacy") this.showLegacyTools = true;
  }

  setSection(s: ProjectSectionV1) {
    this.projectSection = s;
  }

  setError(msg: string | null) {
    this.error = msg;
  }

  setBusy(b: boolean, msg = "") {
    this.busy = b;
    if (msg) this.statusMessage = msg;
  }
}

export const vnextUiStore = new VnextUiStore();
