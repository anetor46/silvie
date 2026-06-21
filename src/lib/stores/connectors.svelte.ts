import {
  deleteIntegration,
  GOOGLE_PROVIDER,
  OUTLOOK_PROVIDER,
  listIntegrations,
  saveIntegration,
  startGoogleOAuth,
  startOutlookOAuth,
  type IntegrationView,
} from '$lib/services/connectors';

class ConnectorStore {
  /** Connected Google integration (Gmail + Calendar) row, if any. */
  google = $state<IntegrationView | null>(null);
  googleLoading = $state(false);
  googleError = $state<string | null>(null);

  /** Connected Outlook integration (Mail + Calendar) row, if any. */
  outlook = $state<IntegrationView | null>(null);
  outlookLoading = $state(false);
  outlookError = $state<string | null>(null);

  loaded = $state(false);

  /** Which mail provider is currently active, if any. */
  get activeMailProvider(): 'google' | 'outlook' | null {
    if (this.google) return 'google';
    if (this.outlook) return 'outlook';
    return null;
  }

  async load(): Promise<void> {
    try {
      const all = await listIntegrations();
      this.google = all.find((i) => i.provider === GOOGLE_PROVIDER) ?? null;
      this.outlook = all.find((i) => i.provider === OUTLOOK_PROVIDER) ?? null;
    } catch (e) {
      console.error('[connectors.load]', e);
    } finally {
      this.loaded = true;
    }
  }

  async connectGoogle(): Promise<void> {
    this.googleLoading = true;
    this.googleError = null;
    try {
      // Mutual exclusion: disconnect Outlook before connecting Google.
      if (this.outlook) {
        await this.disconnectOutlook();
      }
      const tokens = await startGoogleOAuth();
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

  async connectOutlook(): Promise<void> {
    this.outlookLoading = true;
    this.outlookError = null;
    try {
      // Mutual exclusion: disconnect Google before connecting Outlook.
      if (this.google) {
        await this.disconnectGoogle();
      }
      const tokens = await startOutlookOAuth();
      this.outlook = await saveIntegration({
        provider: OUTLOOK_PROVIDER,
        provider_account_id: tokens.provider_account_id,
        provider_account_email: tokens.email,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        scopes: tokens.scopes,
      });
    } catch (e) {
      this.outlookError = e instanceof Error ? e.message : String(e);
    } finally {
      this.outlookLoading = false;
    }
  }

  async disconnectOutlook(): Promise<void> {
    if (!this.outlook) return;
    this.outlookLoading = true;
    this.outlookError = null;
    try {
      await deleteIntegration(this.outlook.id);
      this.outlook = null;
    } catch (e) {
      this.outlookError = e instanceof Error ? e.message : String(e);
    } finally {
      this.outlookLoading = false;
    }
  }

  reset(): void {
    this.google = null;
    this.outlook = null;
    this.loaded = false;
    this.googleError = null;
    this.outlookError = null;
  }
}

export const connectors = new ConnectorStore();
