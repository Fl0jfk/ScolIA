import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  request,
  requestAttr,
  requestAttachment,
  requestComment,
  requestHistory,
} from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import type {
  RequestAttachment,
  RequestComment,
  RequestHistoryItem,
  RequestRecord,
} from "@/app/lib/requests";

function parseTs(raw: string | undefined | null): Date | null {
  if (!raw) return null;
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? null : d;
}

export async function requestsDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}

type AttrBlob = {
  routing?: RequestRecord["routing"];
  parentContext?: RequestRecord["parentContext"];
  assignedExtras?: {
    ccEmails?: string[];
    poolEmails?: string[];
    claimedBy?: RequestRecord["assignedTo"]["claimedBy"];
  };
  commentsMeta?: Record<string, { toRequester?: boolean }>;
  attachmentsMeta?: Record<string, { size?: number; uploadedAt?: string }>;
};

function collectRootAttachments(record: RequestRecord): RequestAttachment[] {
  return [...(record.attachments ?? [])];
}

function collectCommentAttachments(record: RequestRecord): Array<
  RequestAttachment & { commentId: string }
> {
  const out: Array<RequestAttachment & { commentId: string }> = [];
  for (const c of record.comments ?? []) {
    for (const a of c.attachments ?? []) {
      out.push({ ...a, commentId: c.id });
    }
  }
  return out;
}

async function hydrateRequest(
  etablissementId: string,
  m: typeof request.$inferSelect,
): Promise<RequestRecord> {
  const db = getDb();
  const [attrs, comments, history, attachments] = await Promise.all([
    db
      .select()
      .from(requestAttr)
      .where(and(eq(requestAttr.etablissementId, etablissementId), eq(requestAttr.requestId, m.id))),
    db
      .select()
      .from(requestComment)
      .where(
        and(eq(requestComment.etablissementId, etablissementId), eq(requestComment.requestId, m.id)),
      ),
    db
      .select()
      .from(requestHistory)
      .where(
        and(eq(requestHistory.etablissementId, etablissementId), eq(requestHistory.requestId, m.id)),
      ),
    db
      .select()
      .from(requestAttachment)
      .where(
        and(
          eq(requestAttachment.etablissementId, etablissementId),
          eq(requestAttachment.requestId, m.id),
        ),
      ),
  ]);

  const blob = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value }))) as AttrBlob;
  const commentsMeta = blob.commentsMeta ?? {};
  const attachmentsMeta = blob.attachmentsMeta ?? {};

  const sortedComments = [...comments].sort((a, b) => a.sortOrder - b.sortOrder);
  const sortedHistory = [...history].sort((a, b) => a.sortOrder - b.sortOrder);
  const sortedAtts = [...attachments].sort((a, b) => a.sortOrder - b.sortOrder);

  const rootAttachments: RequestAttachment[] = [];
  const byComment = new Map<string, RequestAttachment[]>();

  for (const a of sortedAtts) {
    const meta = attachmentsMeta[a.id] ?? {};
    const item: RequestAttachment = {
      id: a.id,
      key: a.s3Key,
      fileName: a.fileName,
      contentType: a.contentType || "application/octet-stream",
      size: typeof meta.size === "number" ? meta.size : 0,
      uploadedAt: meta.uploadedAt || "",
    };
    if (a.commentId) {
      const list = byComment.get(a.commentId) ?? [];
      list.push(item);
      byComment.set(a.commentId, list);
    } else {
      rootAttachments.push(item);
    }
  }

  const commentsOut: RequestComment[] = sortedComments.map((c) => {
    const meta = commentsMeta[c.id] ?? {};
    return {
      id: c.id,
      at: c.at,
      by: c.byName,
      ...(c.byEmail ? { byEmail: c.byEmail } : {}),
      toRequester: Boolean(meta.toRequester),
      content: c.body,
      ...(byComment.get(c.id)?.length ? { attachments: byComment.get(c.id) } : {}),
    };
  });

  const historyOut: RequestHistoryItem[] = sortedHistory.map((h) => ({
    at: h.at,
    by: h.by,
    action: h.action,
    ...(h.note ? { note: h.note } : {}),
  }));

  const extras = blob.assignedExtras ?? {};

  return {
    id: m.id,
    createdAt: m.createdAt.toISOString(),
    updatedAt: m.updatedAt.toISOString(),
    status: m.status as RequestRecord["status"],
    category: m.category,
    subject: m.subject,
    description: m.description,
    requester: {
      firstName: m.requesterFirstName,
      lastName: m.requesterLastName,
      fullName: m.requesterFullName,
      email: m.requesterEmail,
      phone: m.requesterPhone,
      ...(m.requesterUserId ? { userId: m.requesterUserId } : {}),
    },
    assignedTo: {
      ...(m.assignedRouteId ? { routeId: m.assignedRouteId } : {}),
      unit: m.assignedUnit,
      roleLabel: m.assignedRoleLabel,
      email: m.assignedEmail,
      ...(extras.ccEmails?.length ? { ccEmails: extras.ccEmails } : {}),
      ...(extras.poolEmails?.length ? { poolEmails: extras.poolEmails } : {}),
      ...(extras.claimedBy !== undefined ? { claimedBy: extras.claimedBy } : {}),
    },
    routing: blob.routing ?? {
      source: "fallback",
      confidence: 0,
      reason: "",
    },
    ...(blob.parentContext ? { parentContext: blob.parentContext } : {}),
    ...(rootAttachments.length ? { attachments: rootAttachments } : {}),
    comments: commentsOut,
    history: historyOut,
    ...(m.purgeAt ? { purgeAt: m.purgeAt.toISOString() } : { purgeAt: null }),
  };
}

export async function listRequestsFromDb(etablissementId: string): Promise<RequestRecord[]> {
  const db = getDb();
  const mains = await db.select().from(request).where(eq(request.etablissementId, etablissementId));
  const result: RequestRecord[] = [];
  for (const m of mains) {
    result.push(await hydrateRequest(etablissementId, m));
  }
  return result;
}

export async function getRequestFromDb(
  etablissementId: string,
  id: string,
): Promise<RequestRecord | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(request)
    .where(and(eq(request.etablissementId, etablissementId), eq(request.id, id)))
    .limit(1);
  if (!m) return null;
  return hydrateRequest(etablissementId, m);
}

export async function upsertRequestInDb(
  etablissementId: string,
  record: RequestRecord,
): Promise<void> {
  const db = getDb();
  const id = String(record.id);
  const createdAt = parseTs(record.createdAt) ?? new Date();
  const updatedAt = parseTs(record.updatedAt) ?? new Date();
  const purgeAt = parseTs(record.purgeAt ?? null);

  const main = {
    id,
    etablissementId,
    createdAt,
    updatedAt,
    status: String(record.status),
    category: String(record.category ?? ""),
    subject: String(record.subject ?? ""),
    description: String(record.description ?? ""),
    requesterFirstName: String(record.requester?.firstName ?? ""),
    requesterLastName: String(record.requester?.lastName ?? ""),
    requesterFullName: String(record.requester?.fullName ?? ""),
    requesterEmail: String(record.requester?.email ?? ""),
    requesterPhone: String(record.requester?.phone ?? ""),
    requesterUserId: record.requester?.userId ? String(record.requester.userId) : null,
    assignedUnit: String(record.assignedTo?.unit ?? ""),
    assignedRoleLabel: String(record.assignedTo?.roleLabel ?? ""),
    assignedEmail: String(record.assignedTo?.email ?? ""),
    assignedRouteId: record.assignedTo?.routeId ? String(record.assignedTo.routeId) : null,
    purgeAt,
  };

  await db
    .insert(request)
    .values(main)
    .onConflictDoUpdate({
      target: request.id,
      set: {
        etablissementId: main.etablissementId,
        createdAt: main.createdAt,
        updatedAt: main.updatedAt,
        status: main.status,
        category: main.category,
        subject: main.subject,
        description: main.description,
        requesterFirstName: main.requesterFirstName,
        requesterLastName: main.requesterLastName,
        requesterFullName: main.requesterFullName,
        requesterEmail: main.requesterEmail,
        requesterPhone: main.requesterPhone,
        requesterUserId: main.requesterUserId,
        assignedUnit: main.assignedUnit,
        assignedRoleLabel: main.assignedRoleLabel,
        assignedEmail: main.assignedEmail,
        assignedRouteId: main.assignedRouteId,
        purgeAt: main.purgeAt,
      },
    });

  const commentsMeta: AttrBlob["commentsMeta"] = {};
  for (const c of record.comments ?? []) {
    commentsMeta[c.id] = { toRequester: Boolean(c.toRequester) };
  }
  const attachmentsMeta: AttrBlob["attachmentsMeta"] = {};
  for (const a of collectRootAttachments(record)) {
    attachmentsMeta[a.id] = { size: a.size, uploadedAt: a.uploadedAt };
  }
  for (const a of collectCommentAttachments(record)) {
    attachmentsMeta[a.id] = { size: a.size, uploadedAt: a.uploadedAt };
  }

  const blob: AttrBlob = {
    routing: record.routing,
    ...(record.parentContext ? { parentContext: record.parentContext } : {}),
    assignedExtras: {
      ...(record.assignedTo?.ccEmails?.length ? { ccEmails: record.assignedTo.ccEmails } : {}),
      ...(record.assignedTo?.poolEmails?.length ? { poolEmails: record.assignedTo.poolEmails } : {}),
      ...(record.assignedTo?.claimedBy !== undefined
        ? { claimedBy: record.assignedTo.claimedBy }
        : {}),
    },
    commentsMeta,
    attachmentsMeta,
  };

  await db
    .delete(requestAttr)
    .where(and(eq(requestAttr.etablissementId, etablissementId), eq(requestAttr.requestId, id)));
  const attrs = flattenToAttrs(blob);
  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await db.insert(requestAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          requestId: id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }

  await db
    .delete(requestComment)
    .where(and(eq(requestComment.etablissementId, etablissementId), eq(requestComment.requestId, id)));
  const comments = record.comments ?? [];
  if (comments.length > 0) {
    await db.insert(requestComment).values(
      comments.map((c, i) => ({
        id: c.id,
        etablissementId,
        requestId: id,
        at: c.at || "",
        byName: c.by || "",
        byEmail: c.byEmail || "",
        byUserId: null,
        body: c.content || "",
        sortOrder: i,
      })),
    );
  }

  await db
    .delete(requestHistory)
    .where(and(eq(requestHistory.etablissementId, etablissementId), eq(requestHistory.requestId, id)));
  const hist = record.history ?? [];
  if (hist.length > 0) {
    await db.insert(requestHistory).values(
      hist.map((h, i) => ({
        etablissementId,
        requestId: id,
        at: h.at || "",
        by: h.by || "",
        action: h.action || "",
        note: h.note ? String(h.note) : null,
        sortOrder: i,
      })),
    );
  }

  await db
    .delete(requestAttachment)
    .where(
      and(eq(requestAttachment.etablissementId, etablissementId), eq(requestAttachment.requestId, id)),
    );
  const allAtts = [
    ...collectRootAttachments(record).map((a, i) => ({
      id: a.id,
      etablissementId,
      requestId: id,
      commentId: null as string | null,
      fileName: a.fileName || "",
      s3Key: a.key || "",
      contentType: a.contentType || null,
      sortOrder: i,
    })),
    ...collectCommentAttachments(record).map((a, i) => ({
      id: a.id,
      etablissementId,
      requestId: id,
      commentId: a.commentId,
      fileName: a.fileName || "",
      s3Key: a.key || "",
      contentType: a.contentType || null,
      sortOrder: 1000 + i,
    })),
  ];
  if (allAtts.length > 0) {
    await db.insert(requestAttachment).values(allAtts);
  }
}

export async function deleteRequestFromDb(etablissementId: string, id: string): Promise<void> {
  const db = getDb();
  await db
    .delete(request)
    .where(and(eq(request.etablissementId, etablissementId), eq(request.id, id)));
}

/**
 * Migration paresseuse : si la table typée est vide, reprend l’index collection JSON.
 * Upsert unitaire uniquement — jamais de wipe.
 */
export async function ensureRequestsMigratedFromCollection(
  etablissementId: string,
): Promise<RequestRecord[]> {
  const existing = await listRequestsFromDb(etablissementId);
  if (existing.length > 0) return existing;

  const { getCollectionSingleton } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionSingleton<RequestRecord[]>(etablissementId, "requests__index");
  if (!Array.isArray(legacy) || legacy.length === 0) return [];

  for (const row of legacy) {
    if (row?.id) await upsertRequestInDb(etablissementId, row);
  }
  return listRequestsFromDb(etablissementId);
}
