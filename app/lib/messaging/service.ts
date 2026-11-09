import "server-only";

import {
  and,
  asc,
  count,
  desc,
  eq,
  gt,
  ilike,
  inArray,
  isNotNull,
  isNull,
  lt,
  ne,
  or,
  sql,
} from "drizzle-orm";
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
import { sendWebPushToUsers } from "@/app/lib/web-push/notify";
import type {
  MessagingAttachmentDto,
  MessagingConversationDetailDto,
  MessagingConversationDto,
  MessagingConversationKind,
  MessagingConversationPatchInput,
  MessagingMessageDto,
  MessagingMessageType,
  MessagingPeer,
  MessagingReactionDto,
  MessagingReceiptDetailDto,
  MessagingReceiptStatus,
  MessagingReceiptSummary,
  MessagingSendAttachmentInput,
  MessagingUserDto,
} from "@/app/lib/messaging/types";
import {
  cacheInvalidateMessagingForUsers,
  cacheInvalidateMessagingMessages,
  messagingConversationsCacheKey,
  VALKEY_TTL,
} from "@/app/lib/valkey-cache";
import { valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { valkeyKeyMessagingMessages } from "@/app/lib/valkey-keys";

const DELETED_BODY = "[Message supprimé]";
const MAX_MENTIONS = 20;
const MAX_GROUP_MEMBERS = 200;

async function bustMessagingCaches(
  etablissementId: string,
  conversationId: string,
  userIds: string[],
): Promise<void> {
  await cacheInvalidateMessagingForUsers(etablissementId, userIds);
  await cacheInvalidateMessagingMessages(etablissementId, conversationId);
}

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

type ConversationRow = typeof messagingConversation.$inferSelect;
type ParticipantRow = typeof messagingParticipant.$inferSelect;

function firstName(name: string | null | undefined): string {
  const t = (name ?? "").trim();
  if (!t) return "Utilisateur";
  return t.split(/\s+/)[0] ?? t;
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

/** Vérifie qu’un utilisateur peut être ajouté (même établissement, personnel). */
async function assertEligibleMember(
  etablissementId: string,
  memberId: string,
): Promise<MessagingPeer> {
  const peer = await loadPeer(etablissementId, memberId);
  if (!peer) {
    throw new MessagingError(`Membre introuvable : ${memberId}`, 404, "PEER_NOT_FOUND");
  }
  const { listUserRolesFromDb } = await import("@/app/lib/auth-roles-db");
  const roles = await listUserRolesFromDb(memberId, etablissementId);
  if (!canUseMessaging(roles)) {
    throw new MessagingError(
      `${peer.name} n’est pas joignable via la messagerie.`,
      403,
      "PEER_FORBIDDEN",
    );
  }
  return peer;
}

async function loadUsersByIds(
  etablissementId: string,
  ids: string[],
): Promise<Map<string, MessagingPeer>> {
  const map = new Map<string, MessagingPeer>();
  const unique = [...new Set(ids.filter(Boolean))];
  if (unique.length === 0) return map;
  const db = getDb();
  const rows = await db
    .select({ id: user.id, name: user.name, email: user.email, image: user.image })
    .from(user)
    .where(and(eq(user.etablissementId, etablissementId), inArray(user.id, unique)));
  for (const r of rows) {
    map.set(r.id, { id: r.id, name: r.name, email: r.email, imageUrl: r.image ?? null });
  }
  return map;
}

/**
 * Construit les DTO de conversation pour mes participations (requêtes groupées,
 * pas de N+1) : dernier message, non-lus, mentions, membres.
 */
async function buildConversationDtos(
  etablissementId: string,
  userId: string,
  mine: ParticipantRow[],
): Promise<{
  items: Array<{ dto: MessagingConversationDto; pinnedAt: Date | null }>;
  membersByConv: Map<string, MessagingPeer[]>;
}> {
  const membersByConv = new Map<string, MessagingPeer[]>();
  if (mine.length === 0) return { items: [], membersByConv };
  const db = getDb();
  const conversationIds = mine.map((m) => m.conversationId);
  const partByConv = new Map(mine.map((m) => [m.conversationId, m]));

  const [conversations, lastRows, unreadRows, memberRows] = await Promise.all([
    db
      .select()
      .from(messagingConversation)
      .where(
        and(
          eq(messagingConversation.etablissementId, etablissementId),
          inArray(messagingConversation.id, conversationIds),
        ),
      ),
    db
      .selectDistinctOn([messagingMessage.conversationId], {
        id: messagingMessage.id,
        conversationId: messagingMessage.conversationId,
        body: messagingMessage.body,
        type: messagingMessage.type,
        senderId: messagingMessage.senderId,
        deletedAt: messagingMessage.deletedAt,
        createdAt: messagingMessage.createdAt,
      })
      .from(messagingMessage)
      .where(
        and(
          eq(messagingMessage.etablissementId, etablissementId),
          inArray(messagingMessage.conversationId, conversationIds),
        ),
      )
      .orderBy(messagingMessage.conversationId, desc(messagingMessage.createdAt)),
    db
      .select({
        conversationId: messagingMessage.conversationId,
        unread: count(),
        mentions: sql<number>`count(*) filter (where ${userId}::text = any(${messagingMessage.mentionUserIds}))`,
      })
      .from(messagingMessage)
      .innerJoin(
        messagingParticipant,
        and(
          eq(messagingParticipant.conversationId, messagingMessage.conversationId),
          eq(messagingParticipant.userId, userId),
        ),
      )
      .where(
        and(
          eq(messagingMessage.etablissementId, etablissementId),
          inArray(messagingMessage.conversationId, conversationIds),
          ne(messagingMessage.senderId, userId),
          isNull(messagingMessage.deletedAt),
          gt(messagingMessage.createdAt, messagingParticipant.lastReadAt),
          or(
            isNull(messagingParticipant.hiddenAt),
            gt(messagingMessage.createdAt, messagingParticipant.hiddenAt),
          ),
        ),
      )
      .groupBy(messagingMessage.conversationId),
    db
      .select({
        conversationId: messagingParticipant.conversationId,
        id: user.id,
        name: user.name,
        email: user.email,
        image: user.image,
      })
      .from(messagingParticipant)
      .innerJoin(
        user,
        and(
          eq(user.id, messagingParticipant.userId),
          eq(user.etablissementId, etablissementId),
        ),
      )
      .where(
        and(
          eq(messagingParticipant.etablissementId, etablissementId),
          inArray(messagingParticipant.conversationId, conversationIds),
        ),
      )
      .orderBy(asc(messagingParticipant.createdAt), asc(messagingParticipant.id)),
  ]);

  const lastByConv = new Map(lastRows.map((r) => [r.conversationId, r]));
  const unreadByConv = new Map(
    unreadRows.map((r) => [
      r.conversationId,
      { unread: Number(r.unread ?? 0), mentions: Number(r.mentions ?? 0) },
    ]),
  );
  for (const r of memberRows) {
    const list = membersByConv.get(r.conversationId) ?? [];
    list.push({ id: r.id, name: r.name, email: r.email, imageUrl: r.image ?? null });
    membersByConv.set(r.conversationId, list);
  }

  // Auteurs de dernier message absents de la liste des membres (ex. ont quitté).
  const knownIds = new Set(memberRows.map((r) => r.id));
  const missingSenderIds = lastRows.map((r) => r.senderId).filter((id) => !knownIds.has(id));
  const extraUsers = await loadUsersByIds(etablissementId, missingSenderIds);
  const nameById = new Map<string, string>();
  for (const r of memberRows) nameById.set(r.id, r.name);
  for (const [id, u] of extraUsers) nameById.set(id, u.name);

  const items: Array<{ dto: MessagingConversationDto; pinnedAt: Date | null }> = [];
  for (const conv of conversations) {
    const part = partByConv.get(conv.id);
    if (!part) continue;
    const kind: MessagingConversationKind = conv.kind === "group" ? "group" : "dm";
    const lastRaw = lastByConv.get(conv.id) ?? null;
    // Le dernier message antérieur à ma suppression n’est plus visible pour moi.
    const last =
      lastRaw && part.hiddenAt && lastRaw.createdAt.getTime() <= part.hiddenAt.getTime()
        ? null
        : lastRaw;
    const lastMessage = last
      ? {
          id: last.id,
          body: last.deletedAt ? DELETED_BODY : last.body,
          type: last.type,
          senderId: last.senderId,
          createdAt: last.createdAt.toISOString(),
        }
      : null;
    const counters = unreadByConv.get(conv.id) ?? { unread: 0, mentions: 0 };
    const unreadCount = counters.unread;
    const base = {
      id: conv.id,
      kind,
      lastMessage,
      unreadCount,
      updatedAt: conv.updatedAt.toISOString(),
      lastMessageAt: conv.lastMessageAt ? conv.lastMessageAt.toISOString() : null,
      pinned: part.pinnedAt != null,
      muted: part.muted,
      markedUnread: part.markedUnreadAt != null && unreadCount === 0,
      createdById: conv.createdById ?? null,
      canManage: kind === "group" && conv.createdById === userId,
      lastMessageSenderName: last ? firstName(nameById.get(last.senderId)) : null,
      hasUnreadMention: counters.mentions > 0,
    };
    const members = membersByConv.get(conv.id) ?? [];

    if (kind === "group") {
      items.push({
        pinnedAt: part.pinnedAt ?? null,
        dto: {
          ...base,
          title: (conv.title?.trim() || "Groupe") as string,
          peer: null,
          membersPreview: members.filter((m) => m.id !== userId).slice(0, 3),
          memberCount: members.length,
        },
      });
      continue;
    }

    const peerId = conv.userAId === userId ? (conv.userBId ?? "") : (conv.userAId ?? "");
    const peer = members.find((m) => m.id === peerId) ?? {
      id: peerId || "unknown",
      name: "Utilisateur",
      email: "",
      imageUrl: null,
    };
    items.push({
      pinnedAt: part.pinnedAt ?? null,
      dto: {
        ...base,
        title: peer.name,
        peer,
        membersPreview: [peer],
        memberCount: 2,
      },
    });
  }
  return { items, membersByConv };
}

async function getMyParticipant(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<ParticipantRow> {
  const db = getDb();
  const rows = await db
    .select()
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
  return rows[0];
}

async function getConversationRow(
  etablissementId: string,
  conversationId: string,
): Promise<ConversationRow> {
  const db = getDb();
  const rows = await db
    .select()
    .from(messagingConversation)
    .where(
      and(
        eq(messagingConversation.etablissementId, etablissementId),
        eq(messagingConversation.id, conversationId),
      ),
    )
    .limit(1);
  if (!rows[0]) {
    throw new MessagingError("Conversation introuvable.", 404, "CONVERSATION_NOT_FOUND");
  }
  return rows[0];
}

async function toConversationDto(
  etablissementId: string,
  userId: string,
  conversationId: string,
): Promise<MessagingConversationDto> {
  const part = await getMyParticipant(etablissementId, conversationId, userId);
  const { items } = await buildConversationDtos(etablissementId, userId, [part]);
  const item = items[0];
  if (!item) {
    throw new MessagingError("Conversation introuvable.", 404, "CONVERSATION_NOT_FOUND");
  }
  return item.dto;
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
  await getMyParticipant(etablissementId, conversationId, userId);
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

// ---------------------------------------------------------------------------
// Accusés de réception
// ---------------------------------------------------------------------------

type PeerReceiptCursor = {
  userId: string;
  conversationId: string;
  joinedAt: Date;
  lastReadAt: Date | null;
  lastDeliveredAt: Date | null;
};

function peerDeliveredMs(p: PeerReceiptCursor): number {
  return Math.max(p.lastDeliveredAt?.getTime() ?? 0, p.lastReadAt?.getTime() ?? 0);
}

/** Destinataires concernés : ceux déjà membres à l’envoi du message. */
function peersForMessage(messageCreatedAt: Date, peers: PeerReceiptCursor[]): PeerReceiptCursor[] {
  const createdMs = messageCreatedAt.getTime();
  return peers.filter((p) => p.joinedAt.getTime() <= createdMs);
}

function receiptForMessage(
  messageCreatedAt: Date,
  allPeers: PeerReceiptCursor[],
): MessagingReceiptStatus {
  const peers = peersForMessage(messageCreatedAt, allPeers);
  if (peers.length === 0) return "sent";
  const createdMs = messageCreatedAt.getTime();
  const allRead = peers.every(
    (p) => p.lastReadAt != null && p.lastReadAt.getTime() >= createdMs,
  );
  if (allRead) return "read";
  const allDelivered = peers.every((p) => peerDeliveredMs(p) >= createdMs);
  if (allDelivered) return "delivered";
  return "sent";
}

function receiptSummaryForMessage(
  messageCreatedAt: Date,
  allPeers: PeerReceiptCursor[],
): MessagingReceiptSummary {
  const peers = peersForMessage(messageCreatedAt, allPeers);
  const createdMs = messageCreatedAt.getTime();
  return {
    recipientCount: peers.length,
    deliveredCount: peers.filter((p) => peerDeliveredMs(p) >= createdMs).length,
    readCount: peers.filter(
      (p) => p.lastReadAt != null && p.lastReadAt.getTime() >= createdMs,
    ).length,
  };
}

async function loadPeerReceiptCursors(
  etablissementId: string,
  conversationIds: string[],
  excludeUserId: string,
): Promise<Map<string, PeerReceiptCursor[]>> {
  const map = new Map<string, PeerReceiptCursor[]>();
  if (conversationIds.length === 0) return map;
  const db = getDb();
  const rows = await db
    .select({
      userId: messagingParticipant.userId,
      conversationId: messagingParticipant.conversationId,
      joinedAt: messagingParticipant.createdAt,
      lastReadAt: messagingParticipant.lastReadAt,
      lastDeliveredAt: messagingParticipant.lastDeliveredAt,
    })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        inArray(messagingParticipant.conversationId, conversationIds),
        ne(messagingParticipant.userId, excludeUserId),
      ),
    );
  for (const r of rows) {
    const list = map.get(r.conversationId) ?? [];
    list.push({
      userId: r.userId,
      conversationId: r.conversationId,
      joinedAt: r.joinedAt,
      lastReadAt: r.lastReadAt ?? null,
      lastDeliveredAt: r.lastDeliveredAt ?? null,
    });
    map.set(r.conversationId, list);
  }
  return map;
}

// ---------------------------------------------------------------------------
// Chargement groupé des pièces jointes / réactions / aperçus
// ---------------------------------------------------------------------------

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
      map.set(row.id, DELETED_BODY);
    } else if (row.body?.trim()) {
      map.set(row.id, row.body.slice(0, 160));
    } else {
      map.set(row.id, row.type);
    }
  }
  return map;
}

/**
 * Transforme des lignes de messages en DTO (pièces jointes, réactions, auteur,
 * accusés pour mes messages). Un message supprimé ne renvoie ni corps, ni pièces
 * jointes, ni réactions, ni mentions.
 */
async function hydrateMessages(
  etablissementId: string,
  rows: MessagingMessageRow[],
  viewerId: string | null,
  opts?: { withReceipts?: boolean },
): Promise<MessagingMessageDto[]> {
  if (rows.length === 0) return [];
  const withReceipts = opts?.withReceipts !== false && viewerId != null;
  const liveIds = rows.filter((r) => !r.deletedAt).map((r) => r.id);
  const replyToIds = rows.map((r) => r.replyToId).filter((id): id is string => Boolean(id));
  const mineConvIds = withReceipts
    ? [...new Set(rows.filter((r) => r.senderId === viewerId).map((r) => r.conversationId))]
    : [];

  const [attachmentsMap, reactionsMap, replyPreviews, senders, receiptsByConv] =
    await Promise.all([
      loadAttachmentsForMessages(etablissementId, liveIds),
      loadReactionsForMessages(etablissementId, liveIds),
      loadReplyPreviews(etablissementId, replyToIds),
      loadUsersByIds(etablissementId, [...new Set(rows.map((r) => r.senderId))]),
      loadPeerReceiptCursors(etablissementId, mineConvIds, viewerId ?? ""),
    ]);

  return Promise.all(
    rows.map(async (row) => {
      const deleted = row.deletedAt != null;
      const sender = senders.get(row.senderId);
      const peers = receiptsByConv.get(row.conversationId);
      const dto: MessagingMessageDto = {
        id: row.id,
        conversationId: row.conversationId,
        senderId: row.senderId,
        type: row.type,
        body: deleted ? DELETED_BODY : row.body,
        replyToId: row.replyToId,
        replyPreview: row.replyToId ? (replyPreviews.get(row.replyToId) ?? null) : null,
        forwardedFromId: row.forwardedFromId,
        editedAt: row.editedAt ? row.editedAt.toISOString() : null,
        deletedAt: row.deletedAt ? row.deletedAt.toISOString() : null,
        createdAt: row.createdAt.toISOString(),
        attachments: deleted ? [] : await enrichAttachments(attachmentsMap.get(row.id) ?? []),
        reactions: deleted ? [] : reactionDtos(reactionsMap.get(row.id) ?? []),
        senderName: sender?.name ?? "Utilisateur",
        senderImageUrl: sender?.imageUrl ?? null,
        mentionUserIds: deleted ? [] : (row.mentionUserIds ?? []),
        pinnedAt: deleted || !row.pinnedAt ? null : row.pinnedAt.toISOString(),
        pinnedById: deleted ? null : (row.pinnedById ?? null),
      };
      if (withReceipts && row.senderId === viewerId) {
        const list = peers ?? [];
        dto.receiptStatus = receiptForMessage(row.createdAt, list);
        dto.receiptSummary = receiptSummaryForMessage(row.createdAt, list);
      }
      return dto;
    }),
  );
}

async function hydrateOne(
  etablissementId: string,
  row: MessagingMessageRow,
  viewerId: string | null,
): Promise<MessagingMessageDto> {
  const [dto] = await hydrateMessages(etablissementId, [row], viewerId);
  return dto as MessagingMessageDto;
}

// ---------------------------------------------------------------------------
// Conversations
// ---------------------------------------------------------------------------

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
    void bustMessagingCaches(etablissementId, conversationId, [userAId, userBId]);
  }

  return toConversationDto(etablissementId, userId, conversation.id);
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
  if (cleanTitle.length > 80) {
    throw new MessagingError("Le nom du groupe est trop long (80 caractères max).", 400, "INVALID_TITLE");
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
  if (allIds.length > MAX_GROUP_MEMBERS) {
    throw new MessagingError("Trop de participants pour un groupe.", 400, "GROUP_TOO_LARGE");
  }

  for (const mid of allIds) {
    if (mid === creatorId) continue;
    await assertEligibleMember(etablissementId, mid);
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
  void bustMessagingCaches(etablissementId, conversationId, allIds);

  return toConversationDto(etablissementId, creatorId, conversationId);
}

export async function listConversationsForUser(
  etablissementId: string,
  userId: string,
): Promise<MessagingConversationDto[]> {
  const cacheKey = messagingConversationsCacheKey(etablissementId, userId);
  const cached = await valkeyGetJson<MessagingConversationDto[]>(cacheKey);
  if (cached) return cached;

  const db = getDb();
  const mineAll = await db
    .select()
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.userId, userId),
      ),
    );
  if (mineAll.length === 0) return [];

  // Conversations masquées : visibles de nouveau seulement si un message est arrivé après.
  const hiddenIds = mineAll.filter((p) => p.hiddenAt != null).map((p) => p.conversationId);
  const lastAtByHidden = new Map<string, Date | null>();
  if (hiddenIds.length > 0) {
    const rows = await db
      .select({ id: messagingConversation.id, lastMessageAt: messagingConversation.lastMessageAt })
      .from(messagingConversation)
      .where(
        and(
          eq(messagingConversation.etablissementId, etablissementId),
          inArray(messagingConversation.id, hiddenIds),
        ),
      );
    for (const r of rows) lastAtByHidden.set(r.id, r.lastMessageAt ?? null);
  }
  const mine = mineAll.filter((p) => {
    if (!p.hiddenAt) return true;
    const last = lastAtByHidden.get(p.conversationId);
    return last != null && last.getTime() > p.hiddenAt.getTime();
  });

  const { items } = await buildConversationDtos(etablissementId, userId, mine);
  const sortKey = (d: MessagingConversationDto) =>
    new Date(d.lastMessageAt ?? d.updatedAt).getTime();
  items.sort((a, b) => {
    const pa = a.pinnedAt ? 1 : 0;
    const pb = b.pinnedAt ? 1 : 0;
    if (pa !== pb) return pb - pa;
    return sortKey(b.dto) - sortKey(a.dto);
  });
  const result = items.map((i) => i.dto);
  void valkeySetJson(cacheKey, result, VALKEY_TTL.messagingConversations);
  return result;
}

export async function getConversationDetail(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<MessagingConversationDetailDto> {
  const part = await getMyParticipant(etablissementId, conversationId, userId);
  const { items, membersByConv } = await buildConversationDtos(etablissementId, userId, [part]);
  const item = items[0];
  if (!item) {
    throw new MessagingError("Conversation introuvable.", 404, "CONVERSATION_NOT_FOUND");
  }
  const db = getDb();
  const conditions = [
    eq(messagingMessage.etablissementId, etablissementId),
    eq(messagingMessage.conversationId, conversationId),
    isNotNull(messagingMessage.pinnedAt),
    isNull(messagingMessage.deletedAt),
  ];
  if (part.hiddenAt) conditions.push(gt(messagingMessage.createdAt, part.hiddenAt));
  const pinnedRows = await db
    .select()
    .from(messagingMessage)
    .where(and(...conditions))
    .orderBy(desc(messagingMessage.pinnedAt))
    .limit(50);
  const pinnedMessages = await hydrateMessages(etablissementId, pinnedRows, userId);
  return {
    ...item.dto,
    members: membersByConv.get(conversationId) ?? [],
    pinnedMessages,
  };
}

export async function updateConversationPreferences(
  etablissementId: string,
  conversationId: string,
  userId: string,
  input: MessagingConversationPatchInput,
): Promise<MessagingConversationDetailDto> {
  const part = await getMyParticipant(etablissementId, conversationId, userId);
  const db = getDb();
  const now = new Date();

  const set: Partial<typeof messagingParticipant.$inferInsert> = {};
  if (input.pinned !== undefined) {
    if (typeof input.pinned !== "boolean") {
      throw new MessagingError("Valeur « épinglé » invalide.", 400, "INVALID_INPUT");
    }
    set.pinnedAt = input.pinned ? (part.pinnedAt ?? now) : null;
  }
  if (input.muted !== undefined) {
    if (typeof input.muted !== "boolean") {
      throw new MessagingError("Valeur « muet » invalide.", 400, "INVALID_INPUT");
    }
    set.muted = input.muted;
  }
  if (input.markedUnread !== undefined) {
    if (typeof input.markedUnread !== "boolean") {
      throw new MessagingError("Valeur « non lu » invalide.", 400, "INVALID_INPUT");
    }
    set.markedUnreadAt = input.markedUnread ? now : null;
  }

  let titleChanged = false;
  let newTitle: string | null = null;
  if (input.title !== undefined) {
    if (typeof input.title !== "string") {
      throw new MessagingError("Nom du groupe invalide.", 400, "INVALID_TITLE");
    }
    const conv = await getConversationRow(etablissementId, conversationId);
    if (conv.kind !== "group") {
      throw new MessagingError("Seul un groupe peut être renommé.", 400, "NOT_A_GROUP");
    }
    if (conv.createdById !== userId) {
      throw new MessagingError(
        "Seul le créateur du groupe peut le renommer.",
        403,
        "NOT_MANAGER",
      );
    }
    const title = input.title.trim();
    if (title.length < 2 || title.length > 80) {
      throw new MessagingError(
        "Le nom du groupe doit contenir entre 2 et 80 caractères.",
        400,
        "INVALID_TITLE",
      );
    }
    newTitle = title;
    if (title !== (conv.title ?? "")) {
      await db
        .update(messagingConversation)
        .set({ title, updatedAt: now })
        .where(
          and(
            eq(messagingConversation.etablissementId, etablissementId),
            eq(messagingConversation.id, conversationId),
          ),
        );
      titleChanged = true;
    }
  }

  if (Object.keys(set).length === 0 && input.title === undefined) {
    throw new MessagingError("Aucune modification demandée.", 400, "NOTHING_TO_UPDATE");
  }

  if (Object.keys(set).length > 0) {
    await db
      .update(messagingParticipant)
      .set(set)
      .where(
        and(
          eq(messagingParticipant.etablissementId, etablissementId),
          eq(messagingParticipant.conversationId, conversationId),
          eq(messagingParticipant.userId, userId),
        ),
      );
  }

  if (titleChanged) {
    const userIds = await participantUserIds(etablissementId, conversationId);
    await bustMessagingCaches(etablissementId, conversationId, userIds);
    publish({
      type: "conversation_updated",
      etablissementId,
      userIds,
      payload: { conversationId, title: newTitle },
    });
  } else {
    await bustMessagingCaches(etablissementId, conversationId, [userId]);
    publish({
      type: "conversation_updated",
      etablissementId,
      userIds: [userId],
      payload: { conversationId },
    });
  }

  return getConversationDetail(etablissementId, conversationId, userId);
}

/** « Supprimer pour moi » : masque la conversation et son historique antérieur. */
export async function hideConversationForUser(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<void> {
  await getMyParticipant(etablissementId, conversationId, userId);
  const db = getDb();
  const last = await lastMessageForConversation(etablissementId, conversationId);
  const now = new Date();
  await db
    .update(messagingParticipant)
    .set({
      hiddenAt: now,
      markedUnreadAt: null,
      pinnedAt: null,
      lastReadAt: now,
      lastReadMessageId: last?.id ?? null,
    })
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, userId),
      ),
    );
  await bustMessagingCaches(etablissementId, conversationId, [userId]);
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds: [userId],
    payload: { conversationId, hidden: true },
  });
}

/** Insère un message système (non fourni par le client) et le publie. */
async function postSystemMessage(
  etablissementId: string,
  conversationId: string,
  actorId: string,
  body: string,
  recipientUserIds: string[],
): Promise<void> {
  const db = getDb();
  const now = new Date();
  const row: MessagingMessageRow = {
    id: crypto.randomUUID(),
    etablissementId,
    conversationId,
    senderId: actorId,
    type: "system",
    body,
    replyToId: null,
    forwardedFromId: null,
    editedAt: null,
    deletedAt: null,
    mentionUserIds: [],
    pinnedAt: null,
    pinnedById: null,
    createdAt: now,
  };
  await db.insert(messagingMessage).values(row);
  await db
    .update(messagingConversation)
    .set({ lastMessageAt: now, updatedAt: now })
    .where(
      and(
        eq(messagingConversation.etablissementId, etablissementId),
        eq(messagingConversation.id, conversationId),
      ),
    );
  const dto = await hydrateOne(etablissementId, row, null);
  publish({
    type: "message",
    etablissementId,
    userIds: recipientUserIds,
    payload: { conversationId, message: dto },
  });
}

function joinNames(names: string[]): string {
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} et ${names[names.length - 1]}`;
}

async function assertCanManageGroup(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<ConversationRow> {
  await getMyParticipant(etablissementId, conversationId, userId);
  const conv = await getConversationRow(etablissementId, conversationId);
  if (conv.kind !== "group") {
    throw new MessagingError(
      "Cette action n’est possible que dans un groupe.",
      400,
      "NOT_A_GROUP",
    );
  }
  if (conv.createdById !== userId) {
    throw new MessagingError(
      "Seul le créateur du groupe peut gérer ses membres.",
      403,
      "NOT_MANAGER",
    );
  }
  return conv;
}

export async function addGroupMembers(
  etablissementId: string,
  conversationId: string,
  actorId: string,
  userIds: string[],
): Promise<MessagingConversationDetailDto> {
  await assertCanManageGroup(etablissementId, conversationId, actorId);
  const requested = [...new Set(userIds.map((id) => id.trim()).filter(Boolean))];
  if (requested.length === 0) {
    throw new MessagingError("Aucun membre à ajouter.", 400, "NO_MEMBERS");
  }
  const existingIds = await participantUserIds(etablissementId, conversationId);
  const existing = new Set(existingIds);
  const toAdd = requested.filter((id) => !existing.has(id));
  if (toAdd.length === 0) {
    throw new MessagingError("Ces personnes sont déjà dans le groupe.", 400, "ALREADY_MEMBERS");
  }
  if (existingIds.length + toAdd.length > MAX_GROUP_MEMBERS) {
    throw new MessagingError("Trop de participants pour un groupe.", 400, "GROUP_TOO_LARGE");
  }

  const added: MessagingPeer[] = [];
  for (const id of toAdd) {
    added.push(await assertEligibleMember(etablissementId, id));
  }

  const db = getDb();
  const now = new Date();
  await db.insert(messagingParticipant).values(
    toAdd.map((id) => ({
      id: crypto.randomUUID(),
      etablissementId,
      conversationId,
      userId: id,
      lastReadAt: now,
      lastReadMessageId: null,
      muted: false,
      createdAt: now,
    })),
  );

  const actor = await loadPeer(etablissementId, actorId);
  const allIds = [...existingIds, ...toAdd];
  await postSystemMessage(
    etablissementId,
    conversationId,
    actorId,
    `${actor?.name ?? "Un membre"} a ajouté ${joinNames(added.map((a) => a.name))}`,
    allIds,
  );
  await bustMessagingCaches(etablissementId, conversationId, allIds);
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds: allIds,
    payload: { conversationId },
  });
  return getConversationDetail(etablissementId, conversationId, actorId);
}

/**
 * Quitter (soi-même) ou retirer un membre (créateur). Renvoie le détail pour le
 * demandeur, ou null s’il a quitté le groupe.
 */
export async function removeGroupMember(
  etablissementId: string,
  conversationId: string,
  actorId: string,
  targetUserId: string,
): Promise<MessagingConversationDetailDto | null> {
  const target = targetUserId.trim();
  if (!target) {
    throw new MessagingError("Membre invalide.", 400, "INVALID_USER");
  }
  await getMyParticipant(etablissementId, conversationId, actorId);
  const conv = await getConversationRow(etablissementId, conversationId);
  if (conv.kind !== "group") {
    throw new MessagingError(
      "Impossible de quitter ou de retirer quelqu’un d’une conversation privée.",
      400,
      "NOT_A_GROUP",
    );
  }
  const self = target === actorId;
  if (!self && conv.createdById !== actorId) {
    throw new MessagingError(
      "Seul le créateur du groupe peut retirer un membre.",
      403,
      "NOT_MANAGER",
    );
  }

  const beforeIds = await participantUserIds(etablissementId, conversationId);
  if (!beforeIds.includes(target)) {
    throw new MessagingError("Ce membre ne fait pas partie du groupe.", 404, "NOT_A_MEMBER");
  }

  const [actorPeer, targetPeer] = await Promise.all([
    loadPeer(etablissementId, actorId),
    self ? Promise.resolve(null) : loadPeer(etablissementId, target),
  ]);

  const db = getDb();
  await db
    .delete(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, target),
      ),
    );

  const remaining = beforeIds.filter((id) => id !== target);
  if (remaining.length === 0) {
    // Dernier membre parti : plus personne ne peut lire la conversation.
    await db
      .delete(messagingConversation)
      .where(
        and(
          eq(messagingConversation.etablissementId, etablissementId),
          eq(messagingConversation.id, conversationId),
        ),
      );
    await bustMessagingCaches(etablissementId, conversationId, beforeIds);
    publish({
      type: "conversation_updated",
      etablissementId,
      userIds: beforeIds,
      payload: { conversationId, removed: true },
    });
    return null;
  }

  if (self && conv.createdById === actorId) {
    const oldest = await db
      .select({ userId: messagingParticipant.userId })
      .from(messagingParticipant)
      .where(
        and(
          eq(messagingParticipant.etablissementId, etablissementId),
          eq(messagingParticipant.conversationId, conversationId),
        ),
      )
      .orderBy(asc(messagingParticipant.createdAt), asc(messagingParticipant.id))
      .limit(1);
    const next = oldest[0]?.userId ?? remaining[0] ?? null;
    await db
      .update(messagingConversation)
      .set({ createdById: next, updatedAt: new Date() })
      .where(
        and(
          eq(messagingConversation.etablissementId, etablissementId),
          eq(messagingConversation.id, conversationId),
        ),
      );
  }

  const actorName = actorPeer?.name ?? "Un membre";
  await postSystemMessage(
    etablissementId,
    conversationId,
    actorId,
    self
      ? `${actorName} a quitté le groupe`
      : `${actorName} a retiré ${targetPeer?.name ?? "un membre"}`,
    remaining,
  );
  await bustMessagingCaches(etablissementId, conversationId, beforeIds);
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds: beforeIds,
    payload: { conversationId, ...(self ? {} : { removedUserId: target }) },
  });

  if (self) return null;
  return getConversationDetail(etablissementId, conversationId, actorId);
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

export async function listMessages(
  etablissementId: string,
  conversationId: string,
  userId: string,
  opts?: { cursor?: string; limit?: number },
): Promise<{
  messages: MessagingMessageDto[];
  nextCursor: string | null;
  /** Ma dernière lecture AVANT l'appel (ligne « Nouveaux messages »). */
  myLastReadAt: string | null;
}> {
  const part = await getMyParticipant(etablissementId, conversationId, userId);
  const myLastReadAt = part.lastReadAt ? part.lastReadAt.toISOString() : null;
  const limit = Math.min(Math.max(opts?.limit ?? 40, 1), 100);
  const cursorKey = opts?.cursor?.trim() || "head";
  // Cache uniquement la première page (head) — cursors = navigation rare.
  const cacheKey =
    cursorKey === "head"
      ? valkeyKeyMessagingMessages(etablissementId, conversationId, `u:${userId}:head`)
      : null;
  if (cacheKey) {
    const cached = await valkeyGetJson<{
      messages: MessagingMessageDto[];
      nextCursor: string | null;
    }>(cacheKey);
    if (cached) return { ...cached, myLastReadAt };
  }

  const db = getDb();

  const conditions = [
    eq(messagingMessage.etablissementId, etablissementId),
    eq(messagingMessage.conversationId, conversationId),
  ];
  if (part.hiddenAt) {
    conditions.push(gt(messagingMessage.createdAt, part.hiddenAt));
  }
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

  const messages = await hydrateMessages(etablissementId, page, userId);

  // Un message cité antérieur à ma suppression de la conversation (hiddenAt) :
  // l'aperçu ne doit pas fuiter.
  if (part.hiddenAt) {
    const hiddenMs = part.hiddenAt.getTime();
    const replyIds = [
      ...new Set(messages.map((m) => m.replyToId).filter((x): x is string => Boolean(x))),
    ];
    if (replyIds.length > 0) {
      const targets = await db
        .select({ id: messagingMessage.id, createdAt: messagingMessage.createdAt })
        .from(messagingMessage)
        .where(
          and(
            eq(messagingMessage.etablissementId, etablissementId),
            inArray(messagingMessage.id, replyIds),
          ),
        );
      const old = new Set(
        targets.filter((t) => t.createdAt.getTime() <= hiddenMs).map((t) => t.id),
      );
      for (const m of messages) {
        if (m.replyToId && old.has(m.replyToId)) m.replyPreview = "Message non disponible";
      }
    }
  }

  // Chronologique croissant pour le client
  messages.reverse();

  const oldest = page[page.length - 1];
  const payload = {
    messages,
    nextCursor: hasMore && oldest ? oldest.createdAt.toISOString() : null,
  };
  if (cacheKey) {
    void valkeySetJson(cacheKey, payload, VALKEY_TTL.messagingMessages);
  }
  return { ...payload, myLastReadAt };
}

export type SendMessageInput = {
  body?: string | null;
  type?: string;
  replyToId?: string | null;
  forwardedFromId?: string | null;
  attachments?: MessagingSendAttachmentInput[];
  mentionUserIds?: string[];
};

function pushPreview(
  body: string | null,
  type: MessagingMessageType,
  attachments: MessagingSendAttachmentInput[],
): string {
  const text = (body ?? "").replace(/\s+/g, " ").trim();
  if (text) return text.length > 120 ? `${text.slice(0, 119)}…` : text;
  const isImage =
    type === "image" || attachments.some((a) => a.mime.toLowerCase().startsWith("image/"));
  return isImage ? "📷 Photo" : "📎 Pièce jointe";
}

/** Notification navigateur (best effort) : jamais bloquante, contenu minimal. */
async function notifyNewMessage(args: {
  etablissementId: string;
  conversationId: string;
  senderId: string;
  senderName: string;
  type: MessagingMessageType;
  body: string | null;
  attachments: MessagingSendAttachmentInput[];
  mentionIds: string[];
}): Promise<void> {
  try {
    const db = getDb();
    const [conv, parts] = await Promise.all([
      getConversationRow(args.etablissementId, args.conversationId),
      db
        .select({ userId: messagingParticipant.userId, muted: messagingParticipant.muted })
        .from(messagingParticipant)
        .where(
          and(
            eq(messagingParticipant.etablissementId, args.etablissementId),
            eq(messagingParticipant.conversationId, args.conversationId),
          ),
        ),
    ]);
    const mentioned = new Set(args.mentionIds);
    const others = parts.filter((p) => p.userId !== args.senderId);
    const mentionTargets = others.filter((p) => mentioned.has(p.userId)).map((p) => p.userId);
    const plainTargets = others
      .filter((p) => !mentioned.has(p.userId) && !p.muted)
      .map((p) => p.userId);

    const title =
      conv.kind === "group"
        ? `${args.senderName} · ${conv.title?.trim() || "Groupe"}`
        : args.senderName;
    const base = {
      title,
      body: pushPreview(args.body, args.type, args.attachments),
      url: `/messagerie?c=${args.conversationId}`,
      tag: `msg-${args.conversationId}`,
    };
    const jobs: Promise<void>[] = [];
    if (mentionTargets.length > 0) {
      jobs.push(
        sendWebPushToUsers({
          etablissementId: args.etablissementId,
          userIds: mentionTargets,
          payload: { kind: "messagerie_mention", ...base },
        }),
      );
    }
    if (plainTargets.length > 0) {
      jobs.push(
        sendWebPushToUsers({
          etablissementId: args.etablissementId,
          userIds: plainTargets,
          payload: { kind: "messagerie", ...base },
        }),
      );
    }
    await Promise.all(jobs);
  } catch (error) {
    console.error("[messaging/push]", error);
  }
}

export async function sendMessage(
  etablissementId: string,
  conversationId: string,
  senderId: string,
  input: SendMessageInput,
): Promise<MessagingMessageDto> {
  await assertParticipant(etablissementId, conversationId, senderId);

  const typeRaw = (input.type ?? "text").trim() || "text";
  // Les messages système sont produits par le serveur uniquement.
  if (!isAllowedMessageType(typeRaw) || typeRaw === "system") {
    throw new MessagingError("Type de message invalide.", 400, "INVALID_TYPE");
  }
  const type: MessagingMessageType = typeRaw;
  const attachments = input.attachments ?? [];
  const body = input.body?.trim() || null;

  if (!body && attachments.length === 0 && type === "text") {
    throw new MessagingError("Le message est vide.", 400, "EMPTY_MESSAGE");
  }

  const db = getDb();

  // Transfert : les pièces du message d'origine sont autorisées, à condition d'être
  // participant de sa conversation (sinon on pourrait rattacher le fichier de n'importe qui).
  const forwardedFromId = input.forwardedFromId?.trim() || null;
  const forwardedKeys = new Set<string>();
  if (forwardedFromId) {
    const src = await db
      .select({
        conversationId: messagingMessage.conversationId,
        deletedAt: messagingMessage.deletedAt,
      })
      .from(messagingMessage)
      .where(
        and(
          eq(messagingMessage.etablissementId, etablissementId),
          eq(messagingMessage.id, forwardedFromId),
        ),
      )
      .limit(1);
    const srcRow = src[0];
    if (!srcRow || srcRow.deletedAt) {
      throw new MessagingError("Message à transférer introuvable.", 404, "MESSAGE_NOT_FOUND");
    }
    await assertParticipant(etablissementId, srcRow.conversationId, senderId);
    const srcAttachments = await db
      .select({ s3Key: messagingAttachment.s3Key })
      .from(messagingAttachment)
      .where(
        and(
          eq(messagingAttachment.etablissementId, etablissementId),
          eq(messagingAttachment.messageId, forwardedFromId),
        ),
      );
    for (const a of srcAttachments) forwardedKeys.add(a.s3Key);
  }

  // Validation des pièces jointes avant toute écriture (clé bornée à cette conversation,
  // ou pièce du message transféré).
  const keyPrefix = `messaging/${etablissementId}/${conversationId}/`;
  for (const att of attachments) {
    if (!att.s3Key?.trim() || !att.mime?.trim() || !att.fileName?.trim()) {
      throw new MessagingError("Pièce jointe invalide.", 400, "INVALID_ATTACHMENT");
    }
    if (!Number.isFinite(att.size) || att.size < 0) {
      throw new MessagingError("Taille de pièce jointe invalide.", 400, "INVALID_ATTACHMENT");
    }
    const key = att.s3Key.trim();
    if (key.includes("..") || (!key.includes(keyPrefix) && !forwardedKeys.has(key))) {
      throw new MessagingError("Pièce jointe invalide.", 400, "INVALID_ATTACHMENT");
    }
  }

  if (input.replyToId) {
    const replyRows = await db
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

  const participantIds = await participantUserIds(etablissementId, conversationId);
  const participantSet = new Set(participantIds);
  const mentionIds = [
    ...new Set(
      (input.mentionUserIds ?? [])
        .filter((id): id is string => typeof id === "string")
        .map((id) => id.trim())
        .filter((id) => id && id !== senderId && participantSet.has(id)),
    ),
  ].slice(0, MAX_MENTIONS);

  const now = new Date();
  const messageId = crypto.randomUUID();
  const row: MessagingMessageRow = {
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
    mentionUserIds: mentionIds,
    pinnedAt: null,
    pinnedById: null,
    createdAt: now,
  };
  await db.insert(messagingMessage).values(row);

  for (const att of attachments) {
    await db.insert(messagingAttachment).values({
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
    });
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
    .set({ lastReadAt: now, lastReadMessageId: messageId, markedUnreadAt: null })
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, senderId),
      ),
    );

  const dto = await hydrateOne(etablissementId, row, senderId);

  void bustMessagingCaches(etablissementId, conversationId, participantIds);
  publish({
    type: "message",
    etablissementId,
    userIds: participantIds,
    payload: { conversationId, message: dto },
  });
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds: participantIds,
    payload: { conversationId },
  });

  void notifyNewMessage({
    etablissementId,
    conversationId,
    senderId,
    senderName: dto.senderName,
    type,
    body,
    attachments,
    mentionIds,
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
  if (row.type === "system") {
    throw new MessagingError("Impossible de modifier un message système.", 400, "SYSTEM_MESSAGE");
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

  const dto = await hydrateOne(
    etablissementId,
    { ...row, body: trimmed, editedAt: now },
    userId,
  );

  const userIds = await participantUserIds(etablissementId, row.conversationId);
  void bustMessagingCaches(etablissementId, row.conversationId, userIds);
  publish({
    type: "message",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId, message: dto, edited: true },
  });
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId },
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
  if (row.type === "system") {
    throw new MessagingError("Impossible de supprimer un message système.", 400, "SYSTEM_MESSAGE");
  }
  if (row.deletedAt) {
    throw new MessagingError("Message déjà supprimé.", 400, "ALREADY_DELETED");
  }

  const now = new Date();
  await db
    .update(messagingMessage)
    .set({
      deletedAt: now,
      body: DELETED_BODY,
      pinnedAt: null,
      pinnedById: null,
      mentionUserIds: [],
    })
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    );

  const dto = await hydrateOne(
    etablissementId,
    {
      ...row,
      deletedAt: now,
      body: DELETED_BODY,
      pinnedAt: null,
      pinnedById: null,
      mentionUserIds: [],
    },
    userId,
  );

  const userIds = await participantUserIds(etablissementId, row.conversationId);
  void bustMessagingCaches(etablissementId, row.conversationId, userIds);
  publish({
    type: "message",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId, message: dto, deleted: true },
  });
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId },
  });

  return dto;
}

export async function setMessagePinned(
  etablissementId: string,
  messageId: string,
  userId: string,
  pinned: boolean,
): Promise<MessagingMessageDto> {
  const row = await getMessageForParticipant(etablissementId, messageId, userId);
  if (row.deletedAt) {
    throw new MessagingError("Impossible d’épingler un message supprimé.", 400, "DELETED");
  }
  if (row.type === "system") {
    throw new MessagingError("Impossible d’épingler un message système.", 400, "SYSTEM_MESSAGE");
  }
  const part = await getMyParticipant(etablissementId, row.conversationId, userId);
  if (part.hiddenAt && row.createdAt.getTime() <= part.hiddenAt.getTime()) {
    throw new MessagingError("Message introuvable.", 404, "MESSAGE_NOT_FOUND");
  }

  const db = getDb();
  const now = new Date();
  const next = {
    pinnedAt: pinned ? now : null,
    pinnedById: pinned ? userId : null,
  };
  await db
    .update(messagingMessage)
    .set(next)
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        eq(messagingMessage.id, messageId),
      ),
    );

  const dto = await hydrateOne(etablissementId, { ...row, ...next }, userId);
  const userIds = await participantUserIds(etablissementId, row.conversationId);
  void bustMessagingCaches(etablissementId, row.conversationId, userIds);
  publish({
    type: "conversation_updated",
    etablissementId,
    userIds,
    payload: { conversationId: row.conversationId, messageId, pinned },
  });
  return dto;
}

/** Détail « Vu par » : réservé à l’auteur du message. */
export async function getMessageReceipts(
  etablissementId: string,
  messageId: string,
  userId: string,
): Promise<MessagingReceiptDetailDto> {
  const row = await getMessageForParticipant(etablissementId, messageId, userId);
  if (row.senderId !== userId) {
    throw new MessagingError(
      "Seul l’auteur peut voir qui a lu ce message.",
      403,
      "NOT_SENDER",
    );
  }
  const db = getDb();
  const rows = await db
    .select({
      joinedAt: messagingParticipant.createdAt,
      lastReadAt: messagingParticipant.lastReadAt,
      lastDeliveredAt: messagingParticipant.lastDeliveredAt,
      id: user.id,
      name: user.name,
      email: user.email,
      image: user.image,
    })
    .from(messagingParticipant)
    .innerJoin(
      user,
      and(
        eq(user.id, messagingParticipant.userId),
        eq(user.etablissementId, etablissementId),
      ),
    )
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, row.conversationId),
        ne(messagingParticipant.userId, userId),
      ),
    )
    .orderBy(asc(user.name));

  const createdMs = row.createdAt.getTime();
  const recipients = rows
    .filter((r) => r.joinedAt.getTime() <= createdMs)
    .map((r) => {
      const readMs = r.lastReadAt?.getTime() ?? 0;
      const deliveredMs = Math.max(r.lastDeliveredAt?.getTime() ?? 0, readMs);
      return {
        user: {
          id: r.id,
          name: r.name,
          email: r.email,
          imageUrl: r.image ?? null,
        },
        deliveredAt: deliveredMs >= createdMs ? new Date(deliveredMs).toISOString() : null,
        readAt: r.lastReadAt && readMs >= createdMs ? r.lastReadAt.toISOString() : null,
      };
    });
  return { messageId, recipients };
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
  void cacheInvalidateMessagingMessages(etablissementId, message.conversationId);
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

export async function markDelivered(
  etablissementId: string,
  conversationId: string,
  userId: string,
  messageId?: string | null,
): Promise<{ lastDeliveredAt: string; lastDeliveredMessageId: string | null }> {
  await assertParticipant(etablissementId, conversationId, userId);
  const db = getDb();
  const last = await lastMessageForConversation(etablissementId, conversationId);
  const now = new Date();
  const lastDeliveredMessageId = messageId?.trim() || last?.id || null;

  await db
    .update(messagingParticipant)
    .set({
      lastDeliveredAt: now,
      lastDeliveredMessageId,
    })
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, userId),
      ),
    );

  const userIds = await participantUserIds(etablissementId, conversationId);
  void bustMessagingCaches(etablissementId, conversationId, userIds);
  publish({
    type: "delivered",
    etablissementId,
    userIds,
    payload: {
      conversationId,
      userId,
      lastDeliveredAt: now.toISOString(),
      lastDeliveredMessageId,
    },
  });

  return { lastDeliveredAt: now.toISOString(), lastDeliveredMessageId };
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
    .set({
      lastReadAt: now,
      lastReadMessageId,
      lastDeliveredAt: now,
      lastDeliveredMessageId: lastReadMessageId,
      markedUnreadAt: null,
    })
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, userId),
      ),
    );

  const userIds = await participantUserIds(etablissementId, conversationId);
  void bustMessagingCaches(etablissementId, conversationId, userIds);
  publish({
    type: "delivered",
    etablissementId,
    userIds,
    payload: {
      conversationId,
      userId,
      lastDeliveredAt: now.toISOString(),
      lastDeliveredMessageId: lastReadMessageId,
    },
  });
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
  const pattern = `%${query.replace(/[%_\\]/g, "\\$&")}%`;

  // Jointure sur MA participation : exclut les conversations tierces, les messages
  // antérieurs à ma suppression (hiddenAt), supprimés et système.
  const found = await db
    .select({ m: messagingMessage })
    .from(messagingMessage)
    .innerJoin(
      messagingParticipant,
      and(
        eq(messagingParticipant.conversationId, messagingMessage.conversationId),
        eq(messagingParticipant.userId, userId),
        eq(messagingParticipant.etablissementId, etablissementId),
      ),
    )
    .where(
      and(
        eq(messagingMessage.etablissementId, etablissementId),
        isNull(messagingMessage.deletedAt),
        ne(messagingMessage.type, "system"),
        ilike(messagingMessage.body, pattern),
        or(
          isNull(messagingParticipant.hiddenAt),
          gt(messagingMessage.createdAt, messagingParticipant.hiddenAt),
        ),
      ),
    )
    .orderBy(desc(messagingMessage.createdAt))
    .limit(50);

  return hydrateMessages(
    etablissementId,
    found.map((f) => f.m),
    userId,
    { withReceipts: false },
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
