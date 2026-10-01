import "server-only";

import { and, desc, eq, isNull } from "drizzle-orm";
import { getDb } from "@/db/index";
import { scoliaConversation, scoliaMessage } from "@/db/schema-scolia-chat";

export type ScoliaStoredMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  seq: number;
  createdAt: string;
};

function newId(prefix: string): string {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
}

function titleFromFirstUserMessage(content: string): string {
  const clean = content.replace(/\s+/g, " ").trim();
  if (!clean) return "Nouvelle conversation";
  return clean.length > 72 ? `${clean.slice(0, 69)}…` : clean;
}

export async function listScoliaConversations(opts: {
  etablissementId: string;
  userId: string;
  limit?: number;
}): Promise<
  Array<{
    id: string;
    title: string;
    updatedAt: string;
    lastMessageAt: string | null;
  }>
> {
  const db = getDb();
  const limit = Math.min(Math.max(opts.limit ?? 40, 1), 100);
  const rows = await db
    .select({
      id: scoliaConversation.id,
      title: scoliaConversation.title,
      updatedAt: scoliaConversation.updatedAt,
      lastMessageAt: scoliaConversation.lastMessageAt,
    })
    .from(scoliaConversation)
    .where(
      and(
        eq(scoliaConversation.etablissementId, opts.etablissementId),
        eq(scoliaConversation.userId, opts.userId),
        isNull(scoliaConversation.archivedAt),
      ),
    )
    .orderBy(desc(scoliaConversation.updatedAt))
    .limit(limit);

  return rows.map((r) => ({
    id: r.id,
    title: r.title,
    updatedAt: r.updatedAt.toISOString(),
    lastMessageAt: r.lastMessageAt?.toISOString() ?? null,
  }));
}

export async function getScoliaConversation(opts: {
  etablissementId: string;
  userId: string;
  conversationId: string;
}): Promise<{
  id: string;
  title: string;
  state: Record<string, unknown>;
  messages: ScoliaStoredMessage[];
} | null> {
  const db = getDb();
  const [conv] = await db
    .select()
    .from(scoliaConversation)
    .where(
      and(
        eq(scoliaConversation.etablissementId, opts.etablissementId),
        eq(scoliaConversation.userId, opts.userId),
        eq(scoliaConversation.id, opts.conversationId),
        isNull(scoliaConversation.archivedAt),
      ),
    )
    .limit(1);
  if (!conv) return null;

  const messages = await db
    .select()
    .from(scoliaMessage)
    .where(
      and(
        eq(scoliaMessage.etablissementId, opts.etablissementId),
        eq(scoliaMessage.conversationId, opts.conversationId),
      ),
    )
    .orderBy(scoliaMessage.seq);

  return {
    id: conv.id,
    title: conv.title,
    state: (conv.state && typeof conv.state === "object" ? conv.state : {}) as Record<
      string,
      unknown
    >,
    messages: messages.map((m) => ({
      id: m.id,
      role: m.role,
      content: m.content,
      seq: m.seq,
      createdAt: m.createdAt.toISOString(),
    })),
  };
}

export async function createScoliaConversation(opts: {
  etablissementId: string;
  userId: string;
  title?: string;
  conversationId?: string;
  state?: Record<string, unknown>;
}): Promise<{ id: string; title: string }> {
  const db = getDb();
  const id = opts.conversationId?.trim() || newId("scolia");
  const title = opts.title?.trim() || "Nouvelle conversation";
  await db.insert(scoliaConversation).values({
    id,
    etablissementId: opts.etablissementId,
    userId: opts.userId,
    title,
    state: opts.state ?? {},
  });
  return { id, title };
}

export async function appendScoliaMessages(opts: {
  etablissementId: string;
  userId: string;
  conversationId: string;
  messages: Array<{ role: "user" | "assistant"; content: string }>;
  state?: Record<string, unknown> | null;
  titleHint?: string;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  if (opts.messages.length === 0) return { ok: true };
  const db = getDb();

  const [conv] = await db
    .select({
      id: scoliaConversation.id,
      title: scoliaConversation.title,
    })
    .from(scoliaConversation)
    .where(
      and(
        eq(scoliaConversation.etablissementId, opts.etablissementId),
        eq(scoliaConversation.userId, opts.userId),
        eq(scoliaConversation.id, opts.conversationId),
        isNull(scoliaConversation.archivedAt),
      ),
    )
    .limit(1);

  if (!conv) {
    await createScoliaConversation({
      etablissementId: opts.etablissementId,
      userId: opts.userId,
      conversationId: opts.conversationId,
      title: opts.titleHint ? titleFromFirstUserMessage(opts.titleHint) : undefined,
      state: opts.state ?? {},
    });
  }

  const existing = await db
    .select({ seq: scoliaMessage.seq })
    .from(scoliaMessage)
    .where(eq(scoliaMessage.conversationId, opts.conversationId))
    .orderBy(desc(scoliaMessage.seq))
    .limit(1);
  let nextSeq = (existing[0]?.seq ?? 0) + 1;

  const now = new Date();
  const rows = opts.messages.map((m) => {
    const row = {
      id: newId("smsg"),
      etablissementId: opts.etablissementId,
      conversationId: opts.conversationId,
      role: m.role,
      content: m.content.slice(0, 20_000),
      seq: nextSeq,
      createdAt: now,
    };
    nextSeq += 1;
    return row;
  });

  await db.insert(scoliaMessage).values(rows);

  const patch: {
    updatedAt: Date;
    lastMessageAt: Date;
    state?: Record<string, unknown>;
    title?: string;
  } = {
    updatedAt: now,
    lastMessageAt: now,
  };
  if (opts.state) patch.state = opts.state;
  if (!conv || conv.title === "Nouvelle conversation") {
    const hint =
      opts.titleHint ||
      opts.messages.find((m) => m.role === "user")?.content ||
      "";
    if (hint) patch.title = titleFromFirstUserMessage(hint);
  }

  await db
    .update(scoliaConversation)
    .set(patch)
    .where(
      and(
        eq(scoliaConversation.etablissementId, opts.etablissementId),
        eq(scoliaConversation.userId, opts.userId),
        eq(scoliaConversation.id, opts.conversationId),
      ),
    );

  return { ok: true };
}

export async function archiveScoliaConversation(opts: {
  etablissementId: string;
  userId: string;
  conversationId: string;
}): Promise<boolean> {
  const db = getDb();
  const now = new Date();
  const updated = await db
    .update(scoliaConversation)
    .set({ archivedAt: now, updatedAt: now })
    .where(
      and(
        eq(scoliaConversation.etablissementId, opts.etablissementId),
        eq(scoliaConversation.userId, opts.userId),
        eq(scoliaConversation.id, opts.conversationId),
        isNull(scoliaConversation.archivedAt),
      ),
    )
    .returning({ id: scoliaConversation.id });
  return updated.length > 0;
}

export async function renameScoliaConversation(opts: {
  etablissementId: string;
  userId: string;
  conversationId: string;
  title: string;
}): Promise<boolean> {
  const title = opts.title.replace(/\s+/g, " ").trim().slice(0, 120);
  if (!title) return false;
  const db = getDb();
  const updated = await db
    .update(scoliaConversation)
    .set({ title, updatedAt: new Date() })
    .where(
      and(
        eq(scoliaConversation.etablissementId, opts.etablissementId),
        eq(scoliaConversation.userId, opts.userId),
        eq(scoliaConversation.id, opts.conversationId),
        isNull(scoliaConversation.archivedAt),
      ),
    )
    .returning({ id: scoliaConversation.id });
  return updated.length > 0;
}
