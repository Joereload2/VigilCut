/**
 * Ephemeral player UI state for the shared ShortPreview adapter.
 * Source of media path remains in project/candidate/artifact stores.
 */
class PlayerStore {
  error = $state<string | null>(null);
  ready = $state(false);

  reset() {
    this.error = null;
    this.ready = false;
  }
}

export const playerStore = new PlayerStore();
