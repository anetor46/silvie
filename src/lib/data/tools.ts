/**
 * Registry mapping backend tool names → frontend display metadata.
 *
 * Each entry can carry:
 *   - `label` / `verb` / `icon`: how the card identifies itself
 *   - `widget`: the kind-tag picking which preview component renders the
 *     args for write tools (in both `ConfirmationWidget` and the expanded
 *     `ToolCallCard`). Read tools omit this.
 *   - `brief(args)`: a 1-sentence human-readable description of what the
 *     agent was looking for. Shown in the card's expanded details for read
 *     tools so the user has context without seeing raw JSON.
 *
 * Unknown tool names fall back to a generic entry.
 */

export type WidgetKind =
  | 'email_send'
  | 'email_reply'
  | 'calendar_event_create'
  | 'calendar_event_update'
  | 'calendar_event_delete'
  | 'calendar_event_respond'
  | 'hotel_book';

export interface ToolInfo {
  label: string;
  verb: string;
  icon: string;
  widget?: WidgetKind;
  brief?: (args: Record<string, unknown>) => string;
}

function asString(v: unknown): string | undefined {
  return typeof v === 'string' && v.length > 0 ? v : undefined;
}

function asStringArray(v: unknown): string[] | undefined {
  if (Array.isArray(v) && v.every((s) => typeof s === 'string') && v.length > 0) {
    return v as string[];
  }
  return undefined;
}

/** Format an ISO-ish timestamp as a short human-readable string. Falls
 *  back to the input if parsing fails. */
function fmt(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

function fmtDateOnly(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  } catch {
    return iso;
  }
}

export const TOOL_INFO: Record<string, ToolInfo> = {
  // ── Read tools ──────────────────────────────────────────────────────
  list_emails: {
    label: 'Reading emails',
    verb: 'Reading',
    icon: 'inbox',
    brief: (a) => {
      const q = asString(a.query);
      if (q) return `Searching emails matching “${q}”.`;
      const max = typeof a.max_results === 'number' ? a.max_results : undefined;
      return max
        ? `Reading the latest ${max} emails in your inbox.`
        : 'Reading recent emails from your inbox.';
    },
  },
  get_email: {
    label: 'Opening email',
    verb: 'Opening',
    icon: 'mail',
    brief: () => 'Reading the full body of one email message.',
  },
  list_calendar_events: {
    label: 'Checking calendar',
    verb: 'Checking',
    icon: 'calendar',
    brief: (a) => {
      const q = asString(a.query);
      const start = asString(a.start_time);
      const end = asString(a.end_time);
      if (q && start && end) return `Searching events matching “${q}” between ${fmt(start)} and ${fmt(end)}.`;
      if (q) return `Searching calendar events matching “${q}”.`;
      if (start && end) return `Checking events from ${fmt(start)} to ${fmt(end)}.`;
      if (start) return `Checking events from ${fmt(start)} onward.`;
      return 'Reading upcoming events on your calendar.';
    },
  },
  find_free_time: {
    label: 'Finding free time',
    verb: 'Finding',
    icon: 'clock',
    brief: (a) => {
      const start = asString(a.start_time);
      const end = asString(a.end_time);
      const min = typeof a.min_minutes === 'number' ? a.min_minutes : undefined;
      const range =
        start && end
          ? `between ${fmt(start)} and ${fmt(end)}`
          : start
            ? `from ${fmt(start)}`
            : '';
      const gap = min ? ` (at least ${min} min)` : '';
      const calendars = asStringArray(a.calendars);
      const who =
        calendars && !(calendars.length === 1 && calendars[0] === 'primary')
          ? ` across ${calendars.join(', ')}`
          : '';
      return `Looking for free slots${range ? ' ' + range : ''}${who}${gap}.`.replace(/\s+/g, ' ').trim();
    },
  },
  hotel_search: {
    label: 'Searching hotels',
    verb: 'Searching',
    icon: 'building',
    brief: (a) => {
      const dest = asString(a.destination);
      const checkIn = asString(a.check_in);
      const checkOut = asString(a.check_out);
      const adults = typeof a.adults === 'number' ? a.adults : undefined;
      const guests = adults && adults > 1 ? ` for ${adults} guests` : '';
      if (dest && checkIn && checkOut) {
        return `Searching hotels in ${dest}, ${fmtDateOnly(checkIn)} → ${fmtDateOnly(checkOut)}${guests}.`;
      }
      if (dest) return `Searching hotels in ${dest}${guests}.`;
      return 'Searching for available hotels.';
    },
  },

  // ── Write tools (widget shows the args; no brief needed) ────────────
  send_email: {
    label: 'Sending email',
    verb: 'Sending',
    icon: 'send',
    widget: 'email_send',
  },
  reply_to_email: {
    label: 'Replying to email',
    verb: 'Replying',
    icon: 'reply',
    widget: 'email_reply',
  },
  create_calendar_event: {
    label: 'Creating event',
    verb: 'Creating',
    icon: 'calendar-plus',
    widget: 'calendar_event_create',
  },
  update_calendar_event: {
    label: 'Updating event',
    verb: 'Updating',
    icon: 'calendar-edit',
    widget: 'calendar_event_update',
  },
  delete_calendar_event: {
    label: 'Deleting event',
    verb: 'Deleting',
    icon: 'calendar-x',
    widget: 'calendar_event_delete',
  },
  respond_to_event: {
    label: 'Responding to invite',
    verb: 'Responding',
    icon: 'calendar-check',
    widget: 'calendar_event_respond',
  },
  hotel_book: {
    label: 'Booking hotel',
    verb: 'Booking',
    icon: 'bed',
    widget: 'hotel_book',
  },
};

export function getToolInfo(name: string): ToolInfo {
  return TOOL_INFO[name] ?? { label: name, verb: 'Running', icon: 'tool' };
}
