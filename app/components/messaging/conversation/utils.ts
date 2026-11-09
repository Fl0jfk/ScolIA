import type { MessagingMessageDto, MessagingPeer } from "@/app/lib/messaging/types";

export const GROUP_WINDOW_MS = 5 * 60 * 1000;

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
}

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

export function dayKey(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

export function formatDayLabel(iso: string, now = new Date()): string {
  const d = new Date(iso);
  const diffDays = Math.round((startOfDay(now) - startOfDay(d)) / 86_400_000);
  if (diffDays === 0) return "Aujourd’hui";
  if (diffDays === 1) return "Hier";
  const sameYear = d.getFullYear() === now.getFullYear();
  return d.toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    ...(sameYear ? {} : { year: "numeric" }),
  });
}

export type ListItem =
  | { kind: "day"; key: string; label: string }
  | {
      kind: "message";
      key: string;
      message: MessagingMessageDto;
      firstInGroup: boolean;
      lastInGroup: boolean;
    }
  | { kind: "system"; key: string; message: MessagingMessageDto };

/** Construit la liste affichée : séparateurs de date, regroupement par auteur (< 5 min). */
export function buildItems(messages: MessagingMessageDto[]): ListItem[] {
  const items: ListItem[] = [];
  let prev: MessagingMessageDto | null = null;
  const now = new Date();
  let prevDay: string | null = null;
  for (let i = 0; i < messages.length; i++) {
    const m = messages[i];
    const dk = dayKey(m.createdAt);
    if (dk !== prevDay) {
      items.push({ kind: "day", key: `day-${m.id}`, label: formatDayLabel(m.createdAt, now) });
      prev = null;
      prevDay = dk;
    }
    if (m.type === "system") {
      items.push({ kind: "system", key: m.id, message: m });
      prev = null;
      continue;
    }
    const firstInGroup =
      !prev ||
      prev.senderId !== m.senderId ||
      new Date(m.createdAt).getTime() - new Date(prev.createdAt).getTime() > GROUP_WINDOW_MS;
    const next = messages[i + 1];
    const lastInGroup =
      !next ||
      next.type === "system" ||
      next.senderId !== m.senderId ||
      dayKey(next.createdAt) !== dk ||
      new Date(next.createdAt).getTime() - new Date(m.createdAt).getTime() > GROUP_WINDOW_MS;
    items.push({ kind: "message", key: m.id, message: m, firstInGroup, lastInGroup });
    prev = m;
  }
  return items;
}

export type BodySegment = { text: string; mention: boolean };

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Découpe le texte pour mettre en évidence les « @Prénom Nom » connus. */
export function splitMentions(body: string, names: string[]): BodySegment[] {
  const clean = [...new Set(names.filter(Boolean))].sort((a, b) => b.length - a.length);
  if (clean.length === 0) return [{ text: body, mention: false }];
  const re = new RegExp(`@(?:${clean.map(escapeRegExp).join("|")})`, "gi");
  const out: BodySegment[] = [];
  let last = 0;
  for (const match of body.matchAll(re)) {
    const idx = match.index ?? 0;
    if (idx > last) out.push({ text: body.slice(last, idx), mention: false });
    out.push({ text: match[0], mention: true });
    last = idx + match[0].length;
  }
  if (last < body.length) out.push({ text: body.slice(last), mention: false });
  return out;
}

export function normalizeSearch(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

export function buildMembersMap(
  members: MessagingPeer[] | undefined,
  fallback: Array<MessagingPeer | null | undefined>,
): Map<string, MessagingPeer> {
  const map = new Map<string, MessagingPeer>();
  for (const p of fallback) if (p) map.set(p.id, p);
  for (const p of members ?? []) map.set(p.id, p);
  return map;
}

export function messagePreview(m: MessagingMessageDto): string {
  if (m.deletedAt) return "Message supprimé";
  if (m.body) return m.body;
  if (m.type === "image") return "Photo";
  if (m.type === "audio") return "Message vocal";
  if (m.type === "video") return "Vidéo";
  if (m.attachments.length) return m.attachments[0].fileName;
  return "Message";
}

export function firstName(name: string): string {
  return name.trim().split(/\s+/)[0] || name;
}
