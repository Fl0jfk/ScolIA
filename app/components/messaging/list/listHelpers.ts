import type { MessagingConversationDto, MessagingPeer } from "@/app/lib/messaging/types";

export function conversationTime(iso: string | null | undefined, now = new Date()): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOf(now) - startOf(d)) / 86_400_000);
  if (days <= 0) return d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  if (days === 1) return "Hier";
  if (days < 7) return d.toLocaleDateString("fr-FR", { weekday: "short" });
  return d.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit" });
}

export function isUnread(c: MessagingConversationDto): boolean {
  return c.unreadCount > 0 || c.markedUnread;
}

export function conversationPreview(c: MessagingConversationDto, meId: string): string {
  const m = c.lastMessage;
  if (!m) return "Aucun message";
  let text: string;
  if (m.type === "system") text = m.body?.trim() || "Message système";
  else if (m.type !== "text") text = "\u{1F4CE} Pièce jointe";
  else text = (m.body ?? "").replace(/\s+/g, " ").trim() || "Message supprimé";
  if (m.senderId === meId && m.type !== "system") return `Vous : ${text}`;
  if (c.kind === "group" && m.type !== "system") {
    const first = (c.lastMessageSenderName ?? "").trim().split(/\s+/)[0];
    return first ? `${first} : ${text}` : text;
  }
  return text;
}

export function sortConversations(list: MessagingConversationDto[]): MessagingConversationDto[] {
  const ts = (c: MessagingConversationDto) =>
    new Date(c.lastMessageAt ?? c.updatedAt).getTime() || 0;
  return [...list].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    return ts(b) - ts(a);
  });
}

export function conversationAvatarSource(c: MessagingConversationDto): {
  imageUrl: string | null;
  letter: string;
  name: string;
} {
  if (c.kind === "group") {
    return {
      imageUrl: null,
      letter: (c.title || "G").slice(0, 1).toUpperCase(),
      name: c.title || "Groupe",
    };
  }
  return {
    imageUrl: c.peer?.imageUrl ?? null,
    letter: (c.peer?.name || c.title || "?").slice(0, 1).toUpperCase(),
    name: c.peer?.name || c.title || "?",
  };
}

export function peerLetter(p: MessagingPeer): string {
  return (p.name || "?").slice(0, 1).toUpperCase();
}
