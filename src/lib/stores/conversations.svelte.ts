import type { Message } from '$lib/types';

export interface Conversation {
  id: string;
  title: string;
  messages: Message[];
}

function makeTitle(text: string): string {
  const clean = text.trim().replace(/\s+/g, ' ');
  return clean.length > 40 ? `${clean.slice(0, 40)}…` : clean;
}

class ConversationStore {
  conversations = $state<Conversation[]>([]);
  activeId = $state<string | null>(null);

  get active(): Conversation | undefined {
    return this.conversations.find((c) => c.id === this.activeId);
  }

  /** Called by the chat page when the user sends a message. Creates a new
   *  conversation if there isn't an active one yet. */
  sendUserMessage(content: string): void {
    if (!this.active) {
      const id = crypto.randomUUID();
      this.conversations = [
        { id, title: makeTitle(content), messages: [] },
        ...this.conversations,
      ];
      this.activeId = id;
    }
    const conv = this.active!;
    const nextId = (conv.messages.at(-1)?.id ?? -1) + 1;
    conv.messages.push({ id: nextId, role: 'user', content });
  }

  addAssistantMessage(content: string): void {
    const conv = this.active;
    if (!conv) return;
    const nextId = (conv.messages.at(-1)?.id ?? -1) + 1;
    conv.messages.push({ id: nextId, role: 'assistant', content });
  }

  /** Append an empty assistant message and return its id, so the caller can
   *  stream tokens into it via {@link appendToAssistantMessage}. */
  startAssistantMessage(): number {
    const conv = this.active;
    if (!conv) throw new Error('startAssistantMessage called with no active conversation');
    const nextId = (conv.messages.at(-1)?.id ?? -1) + 1;
    conv.messages.push({ id: nextId, role: 'assistant', content: '' });
    return nextId;
  }

  /** Append a chunk to the assistant message with the given id. Silently no-ops
   *  if the message can't be found (e.g. the conversation was switched). */
  appendToAssistantMessage(id: number, chunk: string): void {
    const conv = this.active;
    if (!conv) return;
    const msg = conv.messages.find((m) => m.id === id);
    if (msg) msg.content += chunk;
  }

  newChat(): void {
    this.activeId = null;
  }

  setActive(id: string): void {
    this.activeId = id;
  }
}

export const conversations = new ConversationStore();
