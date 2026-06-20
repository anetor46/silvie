/** Local view-model for a chat message. Ids are strings (UUIDs from the
 *  backend, or `crypto.randomUUID()` for client-side ones during streaming). */
export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
}
