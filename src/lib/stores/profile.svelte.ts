import { getStoredProfile, saveProfile, type StoredProfile } from '$lib/services/profile';

class ProfileStore {
  data = $state<StoredProfile | null>(null);
  loaded = $state(false);
  loading = $state(false);
  error = $state<string | null>(null);

  async load(): Promise<void> {
    try {
      this.data = await getStoredProfile();
    } catch {
      // keychain may be empty on first launch
    } finally {
      this.loaded = true;
    }
  }

  async save(p: StoredProfile): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      await saveProfile(p);
      this.data = p;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }
}

export const profile = new ProfileStore();
