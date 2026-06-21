/** Local view-model for a chat row. Ids are strings (UUIDs from the
 *  backend, or `crypto.randomUUID()` for client-side ones during streaming).
 *
 *  Three kinds of rows are interleaved in the chat:
 *  - `user` / `assistant`: plain text bubbles
 *  - `tool`: a tool-call card showing icon + label + status, plus an
 *    optional confirmation widget (Approve / Reject) for write actions.
 */
export type MessageRole = 'user' | 'assistant' | 'tool';

export interface Message {
  id: string;
  role: MessageRole;
  content: string;
  /** Present iff role === 'tool'. Parsed from the JSON `content` for stored
   *  rows, or built incrementally during streaming. */
  toolCall?: ToolCallEntry;
}

export type ToolCallStatus = 'pending_user' | 'running' | 'success' | 'error';

export interface ToolCallEntry {
  callId: string;
  name: string;
  args: Record<string, unknown>;
  requiresConfirmation: boolean;
  status: ToolCallStatus;
  summary?: string;
  /** Set once the user clicks Approve / Reject on the confirmation widget.
   *  `null` means the widget is still awaiting input. */
  decision?: 'approved' | 'rejected';
}
