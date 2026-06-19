import {
  startGoogleOAuth,
  getGoogleCalendarAccount,
  disconnectGoogleCalendar,
  type ConnectedAccount,
} from '$lib/services/connectors';

class ConnectorStore {
  googleCalendar = $state<ConnectedAccount | null>(null);
  googleCalendarLoading = $state(false);
  googleCalendarError = $state<string | null>(null);

  async load(): Promise<void> {
    try {
      this.googleCalendar = await getGoogleCalendarAccount();
    } catch {
      // Silently ignore on load — keychain may simply be empty
    }
  }

  async connectGoogleCalendar(): Promise<void> {
    this.googleCalendarLoading = true;
    this.googleCalendarError = null;
    try {
      this.googleCalendar = await startGoogleOAuth();
    } catch (e) {
      this.googleCalendarError = e instanceof Error ? e.message : String(e);
    } finally {
      this.googleCalendarLoading = false;
    }
  }

  async disconnectGoogleCalendar(): Promise<void> {
    this.googleCalendarLoading = true;
    this.googleCalendarError = null;
    try {
      await disconnectGoogleCalendar();
      this.googleCalendar = null;
    } catch (e) {
      this.googleCalendarError = e instanceof Error ? e.message : String(e);
    } finally {
      this.googleCalendarLoading = false;
    }
  }
}

export const connectors = new ConnectorStore();
