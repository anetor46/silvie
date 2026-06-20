import { fetchCurrentUser, syncUser, type User } from '$lib/services/users';

class UserStore {
  record = $state<User | null>(null);
  loaded = $state(false);

  /** GET /users/me. Throws on backend errors. Sets `record = null` on 404. */
  async loadFromBackend(): Promise<void> {
    this.record = await fetchCurrentUser();
    this.loaded = true;
  }

  /** POST /users (find-or-create). Throws on backend errors. */
  async syncFromAuth(name: string, email: string): Promise<void> {
    this.record = await syncUser({ email, name });
    this.loaded = true;
  }

  /** Wipe local state on logout. */
  reset(): void {
    this.record = null;
    this.loaded = false;
  }
}

export const user = new UserStore();
