import "server-only";

import { and, asc, count, desc, eq, gt, ilike, inArray, isNull, lt, ne, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  messagingAttachment,
  messagingConversation,
  messagingMessage,
  messagingParticipant,
  messagingReaction,
  user,
  type MessagingAttachmentRow,
  type MessagingMessageRow,
  type MessagingReactionRow,
} from "@/db/schema";
import { isAllowedMessageType } from "@/app/lib/messaging/constants";
import { publish } from "@/app/lib/messaging/events";
import { listMembersFromDb } from "@/app/lib/members-db";
import { getSignedReadUrl } from "@/app/lib/s3-storage";
import { canUseMessaging } from "@/app/lib/messaging/access";
import type {
  MessagingAttachmentDto,
  MessagingConversationDto,
  MessagingConversationKind,
  MessagingMessageDto,
  MessagingMessageType,
  MessagingPeer,
  MessagingReactionDto,
  MessagingSendAttachmentInput,
  MessagingUserDto,
} from "@/app/lib/messaging/types";

export class MessagingError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(message: string, status = 400, code = "MESSAGING_ERROR") {
    super(message);
    this.name = "MessagingError";
    this.status = status;
    this.code = code;
  }
}

export function normalizePair(a: string, b: string): [string, string] {
  const left = a.trim();
  const right = b.trim();
  if (!left || !right) {
    throw new MessagingError("Identifiants utilisateur invalides.", 400, "INVALID_USER");
  }
  if (left === right) {
    throw new MessagingError("Impossible de démarrer une conversation avec soi-même.", 400, "SAME_USER");
  }
  return left < right ? [left, right] : [right, left];
}

async function loadPeer(
  etablissementId: string,
  peerUserId: string,
): Promise<MessagingPeer | null> {
  if (!peerUserId.trim()) return null;
  const db = getDb();
  const rows = await db
    .select({
      id: user.id,
      name: user.name,
      email: user.email,
      image: user.image,
    })
    .from(user)
    .where(and(eq(user.etablissementId, etablissementId), eq(user.id, peerUserId)))
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  return {
    id: row.id,
    name: row.name,
    email: row.email,
    imageUrl: row.image ?? null,
  };
}

async function loadMembersPreview(
  etablissementId: string,
  conversationId: string,
  excludeUserId?: string,
  limit = 4,
): Promise<{ members: MessagingPeer[]; memberCount: number }> {
  const db = getDb();
  const rows = await db
    .select({ userId: messagingParticipant.userId })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
      ),
    );
  const ids = rows.map((r) => r.userId);
  const memberCount = ids.length;
  const previewIds = ids.filter((id) => id !== excludeUserId).slice(0, limit);
  const members: MessagingPeer[] = [];
  for (const id of previewIds) {
    const peer = await loadPeer(etablissementId, id);
    if (peer) members.push(peer);
  }
  return { members, memberCount };
}

async function toConversationDto(
  etablissementId: string,
  conv: typeof messagingConversation.$inferSelect,
  userId: string,
  lastReadAt: Date | null,
): Promise<MessagingConversationDto> {
  const kind: MessagingConversationKind = conv.kind === "group" ? "group" : "dm";
  const last = await lastMessageForConversation(etablissementId, conv.id);
  const unreadCount = await countUnread(etablissementId, conv.id, userId, lastReadAt);
  const lastMessage = last
    ? {
        id: last.id,
        body: last.deletedAt ? "[Message supprimé]" : last.body,
        type: last.type,
        senderId: last.senderId,
        createdAt: last.createdAt.toISOString(),
      }
    : null;

  if (kind === "group") {
    const { members, memberCount } = await loadMembersPreview(
      etablissementId,
      conv.id,
      userId,
      3,
    );
    return {
      id: conv.id,
      kind,
      title: (conv.title?.trim() || "Groupe") as string,
      peer: null,
      membersPreview: members,
      memberCount,
      lastMessage,
      unreadCount,
      updatedAt: conv.updatedAt.toISOString(),
      lastMessageAt: conv.lastMessageAt ? conv.lastMessageAt.toISOString() : null,
    };
  }

  const peerId =
    conv.userAId === userId ? (conv.userBId ?? "") : (conv.userAId ?? "");
  const peer = (await loadPeer(etablissementId, peerId)) ?? {
    id: peerId || "unknown",
    name: "Utilisateur",
    email: "",
    imageUrl: null,
  };
  return {
    id: conv.id,
    kind: "dm",
    title: peer.name,
    peer,
    membersPreview: [peer],
    memberCount: 2,
    lastMessage,
    unreadCount,
    updatedAt: conv.updatedAt.toISOString(),
    lastMessageAt: conv.lastMessageAt ? conv.lastMessageAt.toISOString() : null,
  };
}


async function participantUserIds(
  etablissementId: string,
  conversationId: string,
): Promise<string[]> {
  const db = getDb();
  const rows = await db
    .select({ userId: messagingParticipant.userId })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
      ),
    );
  return rows.map((r) => r.userId);
}

export async function listConversationParticipantIds(
  etablissementId: string,
  conversationId: string,
): Promise<string[]> {
  return participantUserIds(etablissementId, conversationId);
}

export async function assertParticipant(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<void> {
  const db = getDb();
  const rows = await db
    .select({ id: messagingParticipant.id })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, userId),
      ),
    )
    .limit(1);
  if (!rows[0]) {
    throw new MessagingError("Conversation introuvable ou accès refusé.", 403, "NOT_PARTICIPANT");
  }
}

async function countUnread(
  etablissementId: string,
  conversationId: string,
  userId: string,
  lastReadAt: Date | null,
): Promise<number> {
  const db = getDb();
  const conditions = [
    eq(messagingMessage.etablissementId, etablissementId),
    eq(messagingMessage.conversationId, conversationId),
    ne(messagingMessage.senderId, userId),
    isNull(messagingMessage.deletedAt),
  ];
  if (lastReadAt) {
    conditions.push(gt(messagingMessage.createdAt, lastReadAt));
  }
  const rows = await db
    .select({ value: count() })
    .from(messagingMessage)
    .where(and(...conditions));
  return Number(rows[0]?.value ?? 0);
}

async function lastMessageForConversation(
  etablissementId: string,
  conversationId: string,
): Promise<MessagingMessageRow | null> {
  const db = getDb();
  const rows = await db
    .select()
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.conversationId, conversationId),
      ),
    )
    .orderBy(desc(messagingMessage.createdAt))
    .limit(1);
  return rows[0] ?? null;
}

async function enrichAttachments(
  rows: MessagingAttachmentRow[],
): Promise<MessagingAttachmentDto[]> {
  return Promise.all(
    rows.map(async (row) => ({
      id: row.id,
      s3Key: row.s3Key,
      mime: row.mime,
      size: row.size,
      fileName: row.fileName,
      width: row.width,
      height: row.height,
      url: await getSignedReadUrl(row.s3Key, 3600),
    })),
  );
}

function reactionDtos(rows: MessagingReactionRow[]): MessagingReactionDto[] {
  return rows.map((r) => ({
    emoji: r.emoji,
    userId: r.userId,
  }));
}

async function toMessageDto(
  etablissementId: string,
  row: MessagingMessageRow,
  attachments: MessagingAttachmentRow[],
  reactions: MessagingReactionRow[],
  replyPreview: string | null,
): Promise<MessagingMessageDto> {
  return {
    id: row.id,
    conversationId: row.conversationId,
    senderId: row.senderId,
    type: row.type,
    body: row.body,
    replyToId: row.replyToId,
    replyPreview,
    forwardedFromId: row.forwardedFromId,
    editedAt: row.editedAt ? row.editedAt.toISOString() : null,
    deletedAt: row.deletedAt ? row.deletedAt.toISOString() : null,
    createdAt: row.createdAt.toISOString(),
    attachments: await enrichAttachments(attachments),
    reactions: reactionDtos(reactions),
  };
}

async function loadAttachmentsForMessages(
  etablissementId: string,
  messageIds: string[],
): Promise<Map<string, MessagingAttachmentRow[]>> {
  const map = new Map<string, MessagingAttachmentRow[]>();
  if (messageIds.length === 0) return map;
  const db = getDb();
  const rows = await db
    .select()
    .from(messagingAttachment)
    .where(
      and(
        eq(messagingAttachment.etablissementId, etablissementId),
        inArray(messagingAttachment.messageId, messageIds),
      ),
    )
    .orderBy(asc(messagingAttachment.createdAt));
  for (const row of rows) {
    const list = map.get(row.messageId) ?? [];
    list.push(row);
    map.set(row.messageId, list);
  }
  return map;
}

async function loadReactionsForMessages(
  etablissementId: string,
  messageIds: string[],
): Promise<Map<string, MessagingReactionRow[]>> {
  const map = new Map<string, MessagingReactionRow[]>();
  if (messageIds.length === 0) return map;
  const db = getDb();
  const rows = await db
    .select()
    .from(messagingReaction)
    .where(
      and(
        eq(messagingReaction.etablissementId, etablissementId),
        inArray(messagingReaction.messageId, messageIds),
      ),
    )
    .orderBy(asc(messagingReaction.createdAt));
  for (const row of rows) {
    const list = map.get(row.messageId) ?? [];
    list.push(row);
    map.set(row.messageId, list);
  }
  return map;
}

async function loadReplyPreviews(
  etablissementId: string,
  replyToIds: string[],
): Promise<Map<string, string | null>> {
  const map = new Map<string, string | null>();
  if (replyToIds.length === 0) return map;
  const db = getDb();
  const rows = await db
    .select({
      id: messagingMessage.id,
      body: messagingMessage.body,
      deletedAt: messagingMessage.deletedAt,
      type: messagingMessage.type,
    })
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        inArray(messagingMessage.id, replyToIds),
      ),
    );
  for (const row of rows) {
    if (row.deletedAt) {
      map.set(row.id, "[Message supprimé]");
    } else if (row.body?.trim()) {
      map.set(row.id, row.body.slice(0, 160));
    } else {
      map.set(row.id, row.type);
    }
  }
  return map;
}

export async function getOrCreateConversation(
  etablissementId: string,
  userId: string,
  peerUserId: string,
): Promise<MessagingConversationDto> {
  const [userAId, userBId] = normalizePair(userId, peerUserId);
  const peer = await loadPeer(etablissementId, peerUserId);
  if (!peer) {
    throw new MessagingError("Destinataire introuvable.", 404, "PEER_NOT_FOUND");
  }

  const { listUserRolesFromDb } = await import("@/app/lib/auth-roles-db");
  const peerRoles = await listUserRolesFromDb(peerUserId, etablissementId);
  if (!canUseMessaging(peerRoles)) {
    throw new MessagingError(
      "Ce destinataire n’est pas joignable via la messagerie interne.",
      403,
      "PEER_FORBIDDEN",
    );
  }

  const db = getDb();
  const existing = await db
    .select()
    .from(messagingConversation)
    .where(
      and(
        eq(messagingConversation.etablissementId, etablissementId),
        eq(messagingConversation.userAId, userAId),
        eq(messagingConversation.userBId, userBId),
      ),
    )
    .limit(1);

  let conversation = existing[0];
  if (!conversation) {
    const conversationId = crypto.randomUUID();
    const now = new Date();
    await db.insert(messagingConversation).values({
      id: conversationId,
      etablissementId,
      kind: "dm",
      title: null,
      createdById: userId,
      userAId,
      userBId,
      lastMessageAt: null,
      createdAt: now,
      updatedAt: now,
    });
    await db.insert(messagingParticipant).values([
      {
        id: crypto.randomUUID(),
        etablissementId,
        conversationId,
        userId: userAId,
        lastReadAt: now,
        lastReadMessageId: null,
        muted: false,
        createdAt: now,
      },
      {
        id: crypto.randomUUID(),
        etablissementId,
        conversationId,
        userId: userBId,
        lastReadAt: now,
        lastReadMessageId: null,
        muted: false,
        createdAt: now,
      },
    ]);
    conversation = {
      id: conversationId,
      etablissementId,
      kind: "dm",
      title: null,
      createdById: userId,
      userAId,
      userBId,
      lastMessageAt: null,
      createdAt: now,
      updatedAt: now,
    };

    publish({
      type: "conversation_updated",
      etablissementId,
      userIds: [userAId, userBId],
      payload: { conversationId },
    });
  }

  const me = await db
    .select()
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversation.id),
        eq(messagingParticipant.userId, userId),
      ),
    )
    .limit(1);

  return toConversationDto(
    etablissementId,
    conversation,
    userId,
    me[0]?.lastReadAt ?? null,
  );
}

export async function createGroupConversation(
  etablissementId: string,
  creatorId: string,
  title: string,
  memberIds: string[],
): Promise<MessagingConversationDto> {
  const cleanTitle = title.trim();
  if (cleanTitle.length < 2) {
    throw new MessagingError("Le nom du groupe est trop court.", 400, "INVALID_TITLE");
  }
  const uniqueMembers = [...new Set(memberIds.map((id) => id.trim()).filter(Boolean))];
  const allIds = [...new Set([creatorId, ...uniqueMembers])];
  if (allIds.length < 2) {
    throw new MessagingError(
      "Un groupe nécessite au moins un autre participant.",
      400,
      "GROUP_TOO_SMALL",
    );
  }

  const { listUserRolesFromDb } = await import("@/app/lib/auth-roles-db");
  for (const mid of allIds) {
    if (mid === creatorId) continue;
    const peer = await loadPeer(etablissementId, mid);
    if (!peer) {
      throw new MessagingError(`Membre introuvable : ${mid}`, 404, "PEER_NOT_FOUND");
    }
    const roles = await listUserRolesFromDb(mid, etablissementId);
    if (!canUseMessaging(roles)) {
      throw new MessagingError(
        `${peer.name} n’est pas joignable via la messagerie.`,
        403,
        "PEER_FORBIDDEN",
      );
    }
  }

  const db = getDb();
  const now = new Date();
  const conversationId = crypto.randomUUID();
  await db.insert(messagingConversation).values({
    id: conversationId,
    etablissementId,
    kind: "group",
    title: cleanTitle,
    createdById: creatorId,
    userAId: null,
    userBId: null,
    lastMessageAt: now,
    createdAt: now,
    updatedAt: now,
  });
  for (const mid of allIds) {
    await db.insert(messagingParticipant).values({
      id: crypto.randomUUID(),
      etablissementId,
      conversationId,
      userId: mid,
      lastReadAt: now,
      lastReadMessageId: null,
      muted: false,
      createdAt: now,
    });
  }

  await db.insert(messagingMessage).values({
    id: crypto.randomUUID(),
    etablissementId,
    conversationId,
    senderId: creatorId,
    type: "system",
    body: `Groupe « ${cleanTitle} » créé`,
    replyToId: null,
    forwardedFromId: null,
    editedAt: null,
    deletedAt: null,
    createdAt: now,
  });

  publish({
    type: "conversation_updated",
    etablissementId,
    userIds: allIds,
    payload: { conversationId },
  });

  const conv = (
    await db
      .select()
      .from(messagingConversation)
      .where(
        and(
          eq(messagingConversation.etablissementId, etablissementId),
          eq(messagingConversation.id, conversationId),
        ),
      )
      .limit(1)
  )[0];
  if (!conv) {
    throw new MessagingError("Groupe introuvable après création.", 500, "GROUP_CREATE_FAILED");
  }
  return toConversationDto(etablissementId, conv, creatorId, now);
}

export async function listConversationsForUser(
  etablissementId: string,
  userId: string,
): Promise<MessagingConversationDto[]> {
  const db = getDb();
  const mine = await db
    .select({
      conversationId: messagingParticipant.conversationId,
      lastReadAt: messagingParticipant.lastReadAt,
    })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.userId, userId),
      ),
    );

  if (mine.length === 0) return [];

  const conversationIds = mine.map((m) => m.conversationId);
  const lastReadByConv = new Map(mine.map((m) => [m.conversationId, m.lastReadAt]));

  const conversations = await db
    .select()
    .from(messagingConversation)
    .where(
      and(
        eq(messagingConversation.etablissementId, etablissementId),
        inArray(messagingConversation.id, conversationIds),
      ),
    )
    .orderBy(
      sql`${messagingConversation.lastMessageAt} DESC NULLS LAST`,
      desc(messagingConversation.updatedAt),
    );

  const result: MessagingConversationDto[] = [];
  for (const conv of conversations) {
    result.push(
      await toConversationDto(
        etablissementId,
        conv,
        userId,
        lastReadByConv.get(conv.id) ?? null,
      ),
    );
  }
  return result;
}

export async function listMessages(
  etablissementId: string,
  conversationId: string,
  userId: string,
  opts?: { cursor?: string; limit?: number },
): Promise<{ messages: MessagingMessageDto[]; nextCursor: string | null }> {
  await assertParticipant(etablissementId, conversationId, userId);
  const limit = Math.min(Math.max(opts?.limit ?? 40, 1), 100);
  const db = getDb();

  const conditions = [
    eq(messagingMessage.etablissementId, etablissementId),
    eq(messagingMessage.conversationId, conversationId),
  ];
  if (opts?.cursor?.trim()) {
    const cursorDate = new Date(opts.cursor);
    if (Number.isNaN(cursorDate.getTime())) {
      throw new MessagingError("Curseur de pagination invalide.", 400, "INVALID_CURSOR");
    }
    conditions.push(lt(messagingMessage.createdAt, cursorDate));
  }

  const rows = await db
    .select()
    .from(messagingMessage)
    .where(and(...conditions))
    .orderBy(desc(messagingMessage.createdAt))
    .limit(limit + 1);

  const hasMore = rows.length > limit;
  const page = hasMore ? rows.slice(0, limit) : rows;
  const messageIds = page.map((r) => r.id);
  const replyToIds = page
    .map((r) => r.replyToId)
    .filter((id): id is string => Boolean(id));

  const [attachmentsMap, reactionsMap, replyPreviews] = await Promise.all([
    loadAttachmentsForMessages(etablissementId, messageIds),
    loadReactionsForMessages(etablissementId, messageIds),
    loadReplyPreviews(etablissementId, replyToIds),
  ]);

  const messages = await Promise.all(
    page.map((row) =>
      toMessageDto(
        etablissementId,
        row,
        attachmentsMap.get(row.id) ?? [],
        reactionsMap.get(row.id) ?? [],
        row.replyToId ? (replyPreviews.get(row.replyToId) ?? null) : null,
      ),
    ),
  );

  // Chronologique croissant pour le client
  messages.reverse();

  const oldest = page[page.length - 1];
  return {
    messages,
    nextCursor: hasMore && oldest ? oldest.createdAt.toISOString() : null,
  };
}

export type SendMessageInput = {
  body?: string | null;
  type?: string;
  replyToId?: string | null;
  forwardedFromId?: string | null;
  attachments?: MessagingSendAttachmentInput[];
};

export async function sendMessage(
  etablissementId: string,
  conversationId: string,
  senderId: string,
  input: SendMessageInput,
): Promise<MessagingMessageDto> {
  await assertParticipant(etablissementId, conversationId, senderId);

  const typeRaw = (input.type ?? "text").trim() || "text";
  if (!isAllowedMessageType(typeRaw)) {
    throw new MessagingError("Type de message invalide.", 400, "INVALID_TYPE");
  }
  const type: MessagingMessageType = typeRaw;
  const attachments = input.attachments ?? [];
  const body = input.body?.trim() || null;

  if (!body && attachments.length === 0 && type === "text") {
    throw new MessagingError("Le message est vide.", 400, "EMPTY_MESSAGE");
  }

  if (input.replyToId) {
    const dbCheck = getDb();
    const replyRows = await dbCheck
      .select({ id: messagingMessage.id })
      .from(messagingMessage)
      .where(
        and(
          eq(messagingMessage.etablissementId, etablissementId),
          eq(messagingMessage.conversationId, conversationId),
          eq(messagingMessage.id, input.replyToId),
        ),
      )
      .limit(1);
    if (!replyRows[0]) {
      throw new MessagingError("Message cité introuvable.", 404, "REPLY_NOT_FOUND");
    }
  }

  const db = getDb();
  const now = new Date();
  const messageId = crypto.randomUUID();

  await db.insert(messagingMessage).values({
    id: messageId,
    etablissementId,
    conversationId,
    senderId,
    type,
    body,
    replyToId: input.replyToId?.trim() || null,
    forwardedFromId: input.forwardedFromId?.trim() || null,
    editedAt: null,
    deletedAt: null,
    createdAt: now,
  });

  const attachmentRows: MessagingAttachmentRow[] = [];
  for (const att of attachments) {
    if (!att.s3Key?.trim() || !att.mime?.trim() || !att.fileName?.trim()) {
      throw new MessagingError("Pièce jointe invalide.", 400, "INVALID_ATTACHMENT");
    }
    if (!Number.isFinite(att.size) || att.size < 0) {
      throw new MessagingError("Taille de pièce jointe invalide.", 400, "INVALID_ATTACHMENT");
    }
    const row: MessagingAttachmentRow = {
      id: crypto.randomUUID(),
      etablissementId,
      messageId,
      s3Key: att.s3Key.trim(),
      mime: att.mime.trim(),
      size: Math.floor(att.size),
      fileName: att.fileName.trim(),
      width: att.width ?? null,
      height: att.height ?? null,
      createdAt: now,
    };
    await db.insert(messagingAttachment).values(row);
    attachmentRows.push(row);
  }

  await db
    .update(messagingConversation)
    .set({ lastMessageAt: now, updatedAt: now })
    .where(
      and(
        eq(messagingConversation.etablissementId, etablissementId),
        eq(messagingConversation.id, conversationId),
      ),
    );

  await db
    .update(messagingParticipant)
    .set({ lastReadAt: now, lastReadMessageId: messageId })
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, senderId),
      ),
    );

  const dto = await toMessageDto(
    etablissementId,
    {
      id: messageId,
      etablissementId,
      conversationId,
      senderId,
      type,
      body,
      replyToId: input.replyToId?.trim() || null,
      forwardedFromId: input.forwardedFromId?.trim() || null,
      editedAt: null,
      deletedAt: null,
      createdAt: now,
    },
    attachmentRows,
    [],
    null,
  );

  if (input.replyToId) {
    const previews = await loadReplyPreviews(etablissementId, [input.replyToId]);
    dto.replyPreview = previews.get(input.replyToId) ?? null;
  }

  const userIds = await participantUserIds(etablissementId, conversationId);
  publish({
    type: "message",
    etablissementId,
    userIds,
    payload: { conversationId, message: dto },
  });
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds,
    payload: { conversationId },
  });

  return dto;
}

export async function editMessage(
  etablissementId: string,
  messageId: string,
  userId: string,
  body: string,
): Promise<MessagingMessageDto> {
  const trimmed = body.trim();
  if (!trimmed) {
    throw new MessagingError("Le message est vide.", 400, "EMPTY_MESSAGE");
  }

  const db = getDb();
  const rows = await db
    .select()
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) {
    throw new MessagingError("Message introuvable.", 404, "MESSAGE_NOT_FOUND");
  }
  await assertParticipant(etablissementId, row.conversationId, userId);
  if (row.senderId !== userId) {
    throw new MessagingError("Seul l’auteur peut modifier ce message.", 403, "NOT_SENDER");
  }
  if (row.deletedAt) {
    throw new MessagingError("Impossible de modifier un message supprimé.", 400, "DELETED");
  }

  const now = new Date();
  await db
    .update(messagingMessage)
    .set({ body: trimmed, editedAt: now })
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    );

  const updated: MessagingMessageRow = { ...row, body: trimmed, editedAt: now };
  const [attachmentsMap, reactionsMap, replyPreviews] = await Promise.all([
    loadAttachmentsForMessages(etablissementId, [messageId]),
    loadReactionsForMessages(etablissementId, [messageId]),
    loadReplyPreviews(etablissementId, updated.replyToId ? [updated.replyToId] : []),
  ]);

  const dto = await toMessageDto(
    etablissementId,
    updated,
    attachmentsMap.get(messageId) ?? [],
    reactionsMap.get(messageId) ?? [],
    updated.replyToId ? (replyPreviews.get(updated.replyToId) ?? null) : null,
  );

  const userIds = await participantUserIds(etablissementId, row.conversationId);
  publish({
    type: "message",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId, message: dto, edited: true },
  });

  return dto;
}

export async function softDeleteMessage(
  etablissementId: string,
  messageId: string,
  userId: string,
): Promise<MessagingMessageDto> {
  const db = getDb();
  const rows = await db
    .select()
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) {
    throw new MessagingError("Message introuvable.", 404, "MESSAGE_NOT_FOUND");
  }
  await assertParticipant(etablissementId, row.conversationId, userId);
  if (row.senderId !== userId) {
    throw new MessagingError("Seul l’auteur peut supprimer ce message.", 403, "NOT_SENDER");
  }
  if (row.deletedAt) {
    throw new MessagingError("Message déjà supprimé.", 400, "ALREADY_DELETED");
  }

  const now = new Date();
  const deletedBody = "[Message supprimé]";
  await db
    .update(messagingMessage)
    .set({ deletedAt: now, body: deletedBody })
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    );

  const updated: MessagingMessageRow = { ...row, deletedAt: now, body: deletedBody };
  const [attachmentsMap, reactionsMap] = await Promise.all([
    loadAttachmentsForMessages(etablissementId, [messageId]),
    loadReactionsForMessages(etablissementId, [messageId]),
  ]);

  const dto = await toMessageDto(
    etablissementId,
    updated,
    attachmentsMap.get(messageId) ?? [],
    reactionsMap.get(messageId) ?? [],
    null,
  );

  const userIds = await participantUserIds(etablissementId, row.conversationId);
  publish({
    type: "message",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId, message: dto, deleted: true },
  });

  return dto;
}

export async function setReaction(
  etablissementId: string,
  messageId: string,
  userId: string,
  emoji: string,
): Promise<{ added: boolean; reactions: MessagingReactionDto[] }> {
  const cleaned = emoji.trim();
  if (!cleaned || cleaned.length > 32) {
    throw new MessagingError("Emoji invalide.", 400, "INVALID_EMOJI");
  }

  const db = getDb();
  const rows = await db
    .select()
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
        isNull(messagingMessage.deletedAt),
      ),
    )
    .limit(1);
  const message = rows[0];
  if (!message) {
    throw new MessagingError("Message introuvable.", 404, "MESSAGE_NOT_FOUND");
  }
  await assertParticipant(etablissementId, message.conversationId, userId);

  const existing = await db
    .select()
    .from(messagingReaction)
    .where(
      and(
        eq(messagingReaction.etablissementId, etablissementId),
        eq(messagingReaction.messageId, messageId),
        eq(messagingReaction.userId, userId),
        eq(messagingReaction.emoji, cleaned),
      ),
    )
    .limit(1);

  let added = false;
  if (existing[0]) {
    await db
      .delete(messagingReaction)
      .where(
        and(
          eq(messagingReaction.etablissementId, etablissementId),
          eq(messagingReaction.id, existing[0].id),
        ),
      );
  } else {
    await db.insert(messagingReaction).values({
      id: crypto.randomUUID(),
      etablissementId,
      messageId,
      userId,
      emoji: cleaned,
      createdAt: new Date(),
    });
    added = true;
  }

  const all = await db
    .select()
    .from(messagingReaction)
    .where(
      and(
        eq(messagingReaction.etablissementId, etablissementId),
        eq(messagingReaction.messageId, messageId),
      ),
    )
    .orderBy(asc(messagingReaction.createdAt));

  const reactions = reactionDtos(all);
  const userIds = await participantUserIds(etablissementId, message.conversationId);
  publish({
    type: "reaction",
    etablissementId,
    userIds,
    payload: {
      conversationId: message.conversationId,
      messageId,
      reactions,
      added,
      emoji: cleaned,
      userId,
    },
  });

  return { added, reactions };
}

export async function markRead(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<{ lastReadAt: string; lastReadMessageId: string | null }> {
  await assertParticipant(etablissementId, conversationId, userId);
  const db = getDb();
  const last = await lastMessageForConversation(etablissementId, conversationId);
  const now = new Date();
  const lastReadMessageId = last?.id ?? null;

  await db
    .update(messagingParticipant)
    .set({ lastReadAt: now, lastReadMessageId })
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, userId),
      ),
    );

  const userIds = await participantUserIds(etablissementId, conversationId);
  publish({
    type: "read",
    etablissementId,
    userIds,
    payload: {
      conversationId,
      userId,
      lastReadAt: now.toISOString(),
      lastReadMessageId,
    },
  });

  return { lastReadAt: now.toISOString(), lastReadMessageId };
}

export async function searchMessages(
  etablissementId: string,
  userId: string,
  q: string,
): Promise<MessagingMessageDto[]> {
  const query = q.trim();
  if (query.length < 2) {
    throw new MessagingError("Saisissez au moins 2 caractères.", 400, "QUERY_TOO_SHORT");
  }

  const db = getDb();
  const mine = await db
    .select({ conversationId: messagingParticipant.conversationId })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.userId, userId),
      ),
    );
  if (mine.length === 0) return [];

  const conversationIds = mine.map((m) => m.conversationId);
  const pattern = `%${query.replace(/[%_\\]/g, "\\$&")}%`;

  const rows = await db
    .select()
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        inArray(messagingMessage.conversationId, conversationIds),
        isNull(messagingMessage.deletedAt),
        ilike(messagingMessage.body, pattern),
      ),
    )
    .orderBy(desc(messagingMessage.createdAt))
    .limit(50);

  const messageIds = rows.map((r) => r.id);
  const replyToIds = rows
    .map((r) => r.replyToId)
    .filter((id): id is string => Boolean(id));
  const [attachmentsMap, reactionsMap, replyPreviews] = await Promise.all([
    loadAttachmentsForMessages(etablissementId, messageIds),
    loadReactionsForMessages(etablissementId, messageIds),
    loadReplyPreviews(etablissementId, replyToIds),
  ]);

  return Promise.all(
    rows.map((row) =>
      toMessageDto(
        etablissementId,
        row,
        attachmentsMap.get(row.id) ?? [],
        reactionsMap.get(row.id) ?? [],
        row.replyToId ? (replyPreviews.get(row.replyToId) ?? null) : null,
      ),
    ),
  );
}

export async function listMessagingUsers(
  etablissementId: string,
  excludeUserId: string,
): Promise<MessagingUserDto[]> {
  const members = await listMembersFromDb(etablissementId);
  const db = getDb();
  const images = await db
    .select({ id: user.id, image: user.image })
    .from(user)
    .where(eq(user.etablissementId, etablissementId));
  const imageById = new Map(images.map((u) => [u.id, u.image ?? null]));

  return members
    .map((m) => {
      const id = m.userId?.trim() || m.externalUserId?.trim() || "";
      return { member: m, id };
    })
    .filter((row) => Boolean(row.id) && row.id !== excludeUserId)
    .filter((row) => canUseMessaging(row.member.roles))
    .map((row) => ({
      id: row.id,
      name:
        row.member.displayName ||
        `${row.member.firstName ?? ""} ${row.member.lastName ?? ""}`.trim() ||
        row.member.email,
      email: row.member.email,
      imageUrl: imageById.get(row.id) ?? null,
      roles: row.member.roles,
    }))
    .sort((a, b) => a.name.localeCompare(b.name, "fr"));
}

export async function publishTyping(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<void> {
  await assertParticipant(etablissementId, conversationId, userId);
  const userIds = await participantUserIds(etablissementId, conversationId);
  publish({
    type: "typing",
    etablissementId,
    userIds: userIds.filter((id) => id !== userId),
    payload: { conversationId, userId },
  });
}

/** Charge un message + vérifie participation (routes PATCH/DELETE/reactions). */
export async function getMessageForParticipant(
  etablissementId: string,
  messageId: string,
  userId: string,
): Promise<MessagingMessageRow> {
  const db = getDb();
  const rows = await db
    .select()
    .from(messagingMessage)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) {
    throw new MessagingError("Message introuvable.", 404, "MESSAGE_NOT_FOUND");
  }
  await assertParticipant(etablissementId, row.conversationId, userId);
  return row;
}
