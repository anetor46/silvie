import {
  deleteIntegration,
  GOOGLE_PROVIDER,
  listIntegrations,
  saveIntegration,
  startGoogleOAuth,
  type IntegrationView,
} from '$lib/services/connectors';

class ConnectorStore {
  /** Connected Google integration (Gmail + Calendar) row, if any. */
  google = $state<IntegrationView | null>(null);
  googleLoading = $state(false);
  googleError = $state<string | null>(null);
  loaded = $state(false);

  async load(): Promise<void> {
    try {
      const all = await listIntegrations();
      this.google = all.find((i) => i.provider === GOOGLE_PROVIDER) ?? null;
    } catch (e) {
      // Soft-fail on load — keeps the UI usable when offline / backend down.
      console.error('[connectors.load]', e);
    } finally {
      this.loaded = true;
    }
  }

  async connectGoogle(): Promise<void> {
    this.googleLoading = true;
    this.googleError = null;
    try {
      // 1. Tauri side: open browser, run OAuth, return tokens.
      const tokens = await startGoogleOAuth();
      // 2. Persist on the backend.
      this.google = await saveIntegration({
        provider: GOOGLE_PROVIDER,
        provider_account_id: tokens.provider_account_id,
        provider_account_email: tokens.email,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        scopes: tokens.scopes,
      });
    } catch (e) {
      this.googleError = e instanceof Error ? e.message : String(e);
    } finally {
      this.googleLoading = false;
    }
  }

  async disconnectGoogle(): Promise<void> {
    if (!this.google) return;
    this.googleLoading = true;
    this.googleError = null;
    try {
      await deleteIntegration(this.google.id);
      this.google = null;
    } catch (e) {
      this.googleError = e instanceof Error ? e.message : String(e);
    } finally {
      this.googleLoading = false;
    }
  }

  reset(): void {
    this.google = null;
    this.loaded = false;
    this.googleError = null;
  }
}

export const connectors = new ConnectorStore();
