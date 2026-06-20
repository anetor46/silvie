import {
  fetchUserInfo,
  updateUserInfo,
  type UserInfo,
  type UpdateUserInfoRequest,
} from '$lib/services/user-info';

class UserInfoStore {
  data = $state<UserInfo | null>(null);
  loaded = $state(false);
  loading = $state(false);
  error = $state<string | null>(null);

  /** Fetch /users/me/info. Sets `data` and `loaded`. Throws on backend errors. */
  async load(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.data = await fetchUserInfo();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      this.loaded = true;
      this.loading = false;
    }
  }

  /** PUT /users/me/info with a partial patch. Refreshes `data` on success. */
  async save(req: UpdateUserInfoRequest): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.data = await updateUserInfo(req);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      this.loading = false;
    }
  }

  reset(): void {
    this.data = null;
    this.loaded = false;
    this.error = null;
  }

  clearError(): void {
    this.error = null;
  }
}

export const userInfo = new UserInfoStore();
