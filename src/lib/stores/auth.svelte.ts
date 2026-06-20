import {
  getCurrentUser,
  login as serviceLogin,
  loginBrowser as serviceLoginBrowser,
  logout as serviceLogout,
  requestPasswordReset as serviceResetPassword,
  signup as serviceSignup,
  type AuthUser,
} from '$lib/services/auth';

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

  async login(email: string, password: string): Promise<void> {
    await this.#run(() => serviceLogin(email, password));
  }

  async signup(email: string, password: string, name: string): Promise<void> {
    await this.#run(() => serviceSignup(email, password, name));
  }

  async loginBrowser(connection?: string): Promise<void> {
    await this.#run(() => serviceLoginBrowser(connection));
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
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  clearError(): void {
    this.error = null;
  }

  async #run(fn: () => Promise<AuthUser>): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.user = await fn();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }
}

export const auth = new AuthStore();
