/**
 * Registry mapping backend tool names → frontend display metadata + the
 * widget kind to render in a confirmation card (for write tools).
 *
 * The backend is the source of truth for whether a given call requires
 * confirmation (it sets `requires_confirmation: true` on the SSE event for
 * any tool wrapped in the confirmation harness). The mapping here only
 * decides _which_ widget component renders for the approval UX.
 *
 * Unknown tool names fall back to a generic label and no widget.
 */

export interface ToolInfo {
  /** Card label while running (e.g. "Reading emails"). */
  label: string;
  /** Verb form for the "Awaiting approval" subtitle ("Sending"). */
  verb: string;
  /** SVG icon name — see ToolCallCard.svelte's inline icon map. */
  icon: string;
  /** Widget component to render in the confirmation card. Omit for read tools. */
  widget?:
    | 'email_send'
    | 'email_reply'
    | 'calendar_event_create'
    | 'calendar_event_update'
    | 'calendar_event_delete'
    | 'calendar_event_respond'
    | 'hotel_book';
}

export const TOOL_INFO: Record<string, ToolInfo> = {
  // Read tools
  list_emails: { label: 'Reading emails', verb: 'Reading', icon: 'inbox' },
  get_email: { label: 'Opening email', verb: 'Opening', icon: 'mail' },
  list_calendar_events: {
    label: 'Checking calendar',
    verb: 'Checking',
    icon: 'calendar',
  },
  find_free_time: { label: 'Finding free time', verb: 'Finding', icon: 'clock' },
  hotel_search: { label: 'Searching hotels', verb: 'Searching', icon: 'building' },

  // Write tools
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
