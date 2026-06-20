import {
  deleteIntegration,
  GOOGLE_CALENDAR_PROVIDER,
  listIntegrations,
  saveIntegration,
  startGoogleOAuth,
  type IntegrationView,
} from '$lib/services/connectors';

class ConnectorStore {
  /** Connected Google Calendar integration row, if any. */
  googleCalendar = $state<IntegrationView | null>(null);
  googleCalendarLoading = $state(false);
  googleCalendarError = $state<string | null>(null);
  loaded = $state(false);

  async load(): Promise<void> {
    try {
      const all = await listIntegrations();
      this.googleCalendar =
        all.find((i) => i.provider === GOOGLE_CALENDAR_PROVIDER) ?? null;
    } catch (e) {
      // Soft-fail on load — keeps the UI usable when offline / backend down.
      console.error('[connectors.load]', e);
    } finally {
      this.loaded = true;
    }
  }

  async connectGoogleCalendar(): Promise<void> {
    this.googleCalendarLoading = true;
    this.googleCalendarError = null;
    try {
      // 1. Tauri side: open browser, run OAuth, return tokens.
      const tokens = await startGoogleOAuth();
      // 2. Persist on the backend.
      this.googleCalendar = await saveIntegration({
        provider: GOOGLE_CALENDAR_PROVIDER,
        provider_account_id: tokens.provider_account_id,
        provider_account_email: tokens.email,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        scopes: tokens.scopes,
      });
    } catch (e) {
      this.googleCalendarError = e instanceof Error ? e.message : String(e);
    } finally {
      this.googleCalendarLoading = false;
    }
  }

  async disconnectGoogleCalendar(): Promise<void> {
    if (!this.googleCalendar) return;
    this.googleCalendarLoading = true;
    this.googleCalendarError = null;
    try {
      await deleteIntegration(this.googleCalendar.id);
      this.googleCalendar = null;
    } catch (e) {
      this.googleCalendarError = e instanceof Error ? e.message : String(e);
    } finally {
      this.googleCalendarLoading = false;
    }
  }

  reset(): void {
    this.googleCalendar = null;
    this.loaded = false;
    this.googleCalendarError = null;
  }
}

export const connectors = new ConnectorStore();
