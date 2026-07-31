/**
 * Artifact snapshot slice for vNext MVP.
 */
import type { ArtifactManifestV1 } from "$lib/types/vnext/v1";

class ArtifactStore {
  items = $state<ArtifactManifestV1[]>([]);

  setItems(items: ArtifactManifestV1[]) {
    this.items = items;
  }

  get finalDeliverable(): ArtifactManifestV1 | null {
    return (
      this.items.find(
        (a) =>
          (a.kind === "vertical_mp4" || a.role === "final_deliverable") &&
          a.validationStatus === "passed",
      ) ?? null
    );
  }

  clear() {
    this.items = [];
  }
}

export const artifactStore = new ArtifactStore();
