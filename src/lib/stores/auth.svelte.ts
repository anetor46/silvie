import {
  getCurrentUser,
  login as serviceLogin,
  loginBrowser as serviceLoginBrowser,
  logout as serviceLogout,
  requestPasswordReset as serviceResetPassword,
  signup as serviceSignup,
  type AuthUser,
} from '$lib/services/auth';
import { user } from '$lib/stores/user.svelte';

class AuthStore {
  user = $state<AuthUser | null>(null);
  loading = $state(false);
  loaded = $state(false);
  error = $state<string | null>(null);

  async load(): Promise<void> {
    try {
      this.user = await getCurrentUser();
    } catch {
      // No cached creds — fine, user just isn't logged in.
    } finally {
      this.loaded = true;
    }
  }

  /** Sign in an existing user. Backend must already have a DB row for them. */
  async login(email: string, password: string): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.user = await serviceLogin(email, password);
      await user.loadFromBackend();
      if (!user.record) {
        throw new Error(
          "We couldn't find your account. Please sign up to create one.",
        );
      }
    } catch (e) {
      this.user = null;
      user.reset();
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  /** Create a new Auth0 user, then persist the matching DB row. */
  async signup(email: string, password: string, name: string): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const u = await serviceSignup(email, password, name);
      this.user = u;
      await user.syncFromAuth(name, u.email);
    } catch (e) {
      this.user = null;
      user.reset();
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  /** Browser flow (social / MFA). We don't know if it's signup or login, so
   *  always find-or-create the DB row. */
  async loginBrowser(connection?: string): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const u = await serviceLoginBrowser(connection);
      this.user = u;
      await user.syncFromAuth(u.name, u.email);
    } catch (e) {
      this.user = null;
      user.reset();
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  async requestPasswordReset(email: string): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      await serviceResetPassword(email);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      this.loading = false;
    }
  }

  async logout(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      await serviceLogout();
      this.user = null;
      user.reset();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  clearError(): void {
    this.error = null;
  }
}

export const auth = new AuthStore();
