import "server-only";

import { and, asc, desc, eq, inArray, isNull, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  familleMessagingSettings,
  familleNotif,
  familleThread,
  familleThreadAttachment,
  familleThreadMessage,
  foyer,
} from "@/db/schema";
import {
  DEFAULT_FAMILLE_MESSAGING_SETTINGS,
  type FamilleMessagingSettingsDto,
} from "@/app/lib/famille-messaging-matrix";

export type FamilleThreadRow = typeof familleThread.$inferSelect;
export type FamilleThreadMessageRow = typeof familleThreadMessage.$inferSelect;
export type FamilleAttachmentRow = typeof familleThreadAttachment.$inferSelect;

export type FamilleAttachmentInput = {
  fileName: string;
  mime: string;
  size: number;
  s3Key?: string | null;
  contentBase64?: string | null;
};

function trimCorps(v: string): string {
  return v.trim().slice(0, 4000);
}

export async function listFoyersLight(etablissementId: string) {
  const db = getDb();
  const rows = await db
    .select({
      foyerId: foyer.id,
      foyerLabel: foyer.label,
      eleveId: eleve.id,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
    })
    .from(foyer)
    .leftJoin(
      eleveFoyerLink,
      and(
        eq(eleveFoyerLink.foyerId, foyer.id),
        eq(eleveFoyerLink.etablissementId, etablissementId),
      ),
    )
    .leftJoin(
      eleve,
      and(eq(eleve.id, eleveFoyerLink.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(eq(foyer.etablissementId, etablissementId))
    .orderBy(asc(foyer.label), asc(eleve.nom))
    .limit(300);

  type FoyerLight = {
    id: string;
    label: string;
    eleves: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
  };
  const byId = new Map<string, FoyerLight>();
  for (const r of rows) {
    let f = byId.get(r.foyerId);
    if (!f) {
      f = { id: r.foyerId, label: r.foyerLabel || "Foyer", eleves: [] };
      byId.set(r.foyerId, f);
    }
    if (r.eleveId) {
      f.eleves.push({
        id: r.eleveId,
        nom: r.eleveNom || "",
        prenom: r.elevePrenom || "",
        classe: r.eleveClasse,
      });
    }
  }
  return [...byId.values()];
}

export async function getFamilleMessagingSettings(
  etablissementId: string,
): Promise<FamilleMessagingSettingsDto> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(familleMessagingSettings)
    .where(eq(familleMessagingSettings.etablissementId, etablissementId))
    .limit(1);
  if (!row) return { ...DEFAULT_FAMILLE_MESSAGING_SETTINGS };
  return {
    rolesCanInitiate: Array.isArray(row.rolesCanInitiate)
      ? row.rolesCanInitiate.map(String)
      : [...DEFAULT_FAMILLE_MESSAGING_SETTINGS.rolesCanInitiate],
    profOwnClassesOnly: row.profOwnClassesOnly !== false,
    allowBroadcast: row.allowBroadcast !== false,
    allowParentAttachments: row.allowParentAttachments !== false,
  };
}

export async function upsertFamilleMessagingSettings(
  etablissementId: string,
  input: FamilleMessagingSettingsDto,
  updatedByUserId: string,
): Promise<FamilleMessagingSettingsDto> {
  const roles = [...new Set(input.rolesCanInitiate.map((r) => r.trim()).filter(Boolean))];
  if (!roles.length) throw new Error("Au moins un rôle initiateur est requis.");
  const db = getDb();
  const values = {
    etablissementId,
    rolesCanInitiate: roles,
    profOwnClassesOnly: !!input.profOwnClassesOnly,
    allowBroadcast: !!input.allowBroadcast,
    allowParentAttachments: !!input.allowParentAttachments,
    updatedAt: new Date(),
    updatedByUserId,
  };
  await db
    .insert(familleMessagingSettings)
    .values(values)
    .onConflictDoUpdate({
      target: familleMessagingSettings.etablissementId,
      set: {
        rolesCanInitiate: values.rolesCanInitiate,
        profOwnClassesOnly: values.profOwnClassesOnly,
        allowBroadcast: values.allowBroadcast,
        allowParentAttachments: values.allowParentAttachments,
        updatedAt: values.updatedAt,
        updatedByUserId: values.updatedByUserId,
      },
    });
  return getFamilleMessagingSettings(etablissementId);
}

export async function listStaffFamilleThreads(etablissementId: string) {
  const db = getDb();
  return db
    .select({
      id: familleThread.id,
      foyerId: familleThread.foyerId,
      eleveId: familleThread.eleveId,
      sujet: familleThread.sujet,
      isBroadcast: familleThread.isBroadcast,
      createdByNom: familleThread.createdByNom,
      lastMessageAt: familleThread.lastMessageAt,
      createdAt: familleThread.createdAt,
      foyerLabel: foyer.label,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
    })
    .from(familleThread)
    .leftJoin(foyer, eq(foyer.id, familleThread.foyerId))
    .leftJoin(eleve, eq(eleve.id, familleThread.eleveId))
    .where(eq(familleThread.etablissementId, etablissementId))
    .orderBy(desc(familleThread.lastMessageAt))
    .limit(100);
}

export async function listFamilleThreadsForFoyers(
  etablissementId: string,
  foyerIds: string[],
) {
  if (!foyerIds.length) return [];
  const db = getDb();
  return db
    .select({
      id: familleThread.id,
      foyerId: familleThread.foyerId,
      eleveId: familleThread.eleveId,
      sujet: familleThread.sujet,
      isBroadcast: familleThread.isBroadcast,
      createdByNom: familleThread.createdByNom,
      lastMessageAt: familleThread.lastMessageAt,
      createdAt: familleThread.createdAt,
      foyerLabel: foyer.label,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
    })
    .from(familleThread)
    .leftJoin(foyer, eq(foyer.id, familleThread.foyerId))
    .leftJoin(eleve, eq(eleve.id, familleThread.eleveId))
    .where(
      and(
        eq(familleThread.etablissementId, etablissementId),
        inArray(familleThread.foyerId, foyerIds),
      ),
    )
    .orderBy(desc(familleThread.lastMessageAt))
    .limit(100);
}

export async function getFamilleThread(
  etablissementId: string,
  threadId: string,
): Promise<FamilleThreadRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(familleThread)
    .where(
      and(eq(familleThread.etablissementId, etablissementId), eq(familleThread.id, threadId)),
    )
    .limit(1);
  return row ?? null;
}

export async function listFamilleThreadMessages(
  etablissementId: string,
  threadId: string,
): Promise<FamilleThreadMessageRow[]> {
  const db = getDb();
  return db
    .select()
    .from(familleThreadMessage)
    .where(
      and(
        eq(familleThreadMessage.etablissementId, etablissementId),
        eq(familleThreadMessage.threadId, threadId),
      ),
    )
    .orderBy(asc(familleThreadMessage.createdAt));
}

export async function listAttachmentsForMessages(
  etablissementId: string,
  messageIds: string[],
): Promise<FamilleAttachmentRow[]> {
  if (!messageIds.length) return [];
  const db = getDb();
  return db
    .select()
    .from(familleThreadAttachment)
    .where(
      and(
        eq(familleThreadAttachment.etablissementId, etablissementId),
        inArray(familleThreadAttachment.messageId, messageIds),
      ),
    )
    .orderBy(asc(familleThreadAttachment.createdAt));
}

async function insertAttachments(
  etablissementId: string,
  messageId: string,
  attachments: FamilleAttachmentInput[],
) {
  if (!attachments.length) return [];
  const db = getDb();
  const rows: FamilleAttachmentRow[] = [];
  for (const att of attachments) {
    const [row] = await db
      .insert(familleThreadAttachment)
      .values({
        etablissementId,
        messageId,
        fileName: att.fileName.slice(0, 200),
        mime: att.mime.slice(0, 120),
        size: Math.max(0, Number(att.size) || 0),
        s3Key: att.s3Key || null,
        contentBase64: att.contentBase64 || null,
      })
      .returning();
    if (row) rows.push(row);
  }
  return rows;
}

export async function createFamilleNotif(input: {
  etablissementId: string;
  foyerId: string;
  threadId: string;
  kind: "new_message" | "broadcast" | "reply_staff";
  titre: string;
  preview: string;
}) {
  const db = getDb();
  const [row] = await db
    .insert(familleNotif)
    .values({
      etablissementId: input.etablissementId,
      foyerId: input.foyerId,
      threadId: input.threadId,
      kind: input.kind,
      titre: input.titre.slice(0, 200),
      preview: input.preview.slice(0, 280),
    })
    .returning();
  return row ?? null;
}

export async function createFamilleThreadWithMessage(
  etablissementId: string,
  input: {
    foyerId: string;
    eleveId?: string | null;
    sujet: string;
    corps: string;
    auteurUserId: string;
    auteurNom: string;
    isBroadcast?: boolean;
    attachments?: FamilleAttachmentInput[];
  },
) {
  const sujet = input.sujet.trim().slice(0, 200);
  const corps = trimCorps(input.corps);
  const attachments = input.attachments || [];
  if (!sujet) throw new Error("Sujet obligatoire.");
  if (!corps && !attachments.length) throw new Error("Message ou pièce obligatoire.");
  if (!input.foyerId.trim()) throw new Error("Foyer obligatoire.");

  const db = getDb();
  const now = new Date();
  const [thread] = await db
    .insert(familleThread)
    .values({
      etablissementId,
      foyerId: input.foyerId.trim(),
      eleveId: input.eleveId?.trim() || null,
      sujet,
      isBroadcast: !!input.isBroadcast,
      createdByUserId: input.auteurUserId,
      createdByNom: input.auteurNom.slice(0, 120),
      lastMessageAt: now,
      createdAt: now,
      updatedAt: now,
    })
    .returning();
  if (!thread) throw new Error("Création thread impossible.");

  const [message] = await db
    .insert(familleThreadMessage)
    .values({
      etablissementId,
      threadId: thread.id,
      auteurCote: "staff",
      auteurUserId: input.auteurUserId,
      auteurNom: input.auteurNom.slice(0, 120),
      corps: corps || (attachments.length ? `(${attachments.length} pièce(s))` : ""),
      createdAt: now,
    })
    .returning();
  if (!message) throw new Error("Création message impossible.");

  const attRows = await insertAttachments(etablissementId, message.id, attachments);

  await createFamilleNotif({
    etablissementId,
    foyerId: thread.foyerId,
    threadId: thread.id,
    kind: input.isBroadcast ? "broadcast" : "new_message",
    titre: sujet,
    preview: corps || attRows.map((a) => a.fileName).join(", "),
  });

  return { thread, message, attachments: attRows };
}

/** Diffusion : un thread + 1er message par foyer. */
export async function broadcastFamilleMessage(
  etablissementId: string,
  input: {
    foyerIds: string[];
    sujet: string;
    corps: string;
    auteurUserId: string;
    auteurNom: string;
    attachments?: FamilleAttachmentInput[];
  },
) {
  const foyerIds = [...new Set(input.foyerIds.map((id) => id.trim()).filter(Boolean))];
  if (!foyerIds.length) throw new Error("Aucun foyer pour la diffusion.");
  const created = [];
  for (const foyerId of foyerIds) {
    created.push(
      await createFamilleThreadWithMessage(etablissementId, {
        foyerId,
        sujet: input.sujet,
        corps: input.corps,
        auteurUserId: input.auteurUserId,
        auteurNom: input.auteurNom,
        isBroadcast: true,
        attachments: input.attachments,
      }),
    );
  }
  return created;
}

export async function replyFamilleThreadMessage(
  etablissementId: string,
  input: {
    threadId: string;
    auteurCote: "staff" | "parent";
    auteurUserId: string;
    auteurNom: string;
    corps: string;
    attachments?: FamilleAttachmentInput[];
  },
) {
  const corps = trimCorps(input.corps);
  const attachments = input.attachments || [];
  if (!corps && !attachments.length) throw new Error("Message ou pièce obligatoire.");
  const thread = await getFamilleThread(etablissementId, input.threadId);
  if (!thread) throw new Error("Conversation introuvable.");

  const db = getDb();
  const now = new Date();
  const [message] = await db
    .insert(familleThreadMessage)
    .values({
      etablissementId,
      threadId: thread.id,
      auteurCote: input.auteurCote,
      auteurUserId: input.auteurUserId,
      auteurNom: input.auteurNom.slice(0, 120),
      corps: corps || (attachments.length ? `(${attachments.length} pièce(s))` : ""),
      createdAt: now,
    })
    .returning();
  if (!message) throw new Error("Envoi impossible.");

  const attRows = await insertAttachments(etablissementId, message.id, attachments);

  await db
    .update(familleThread)
    .set({ lastMessageAt: now, updatedAt: now })
    .where(
      and(eq(familleThread.etablissementId, etablissementId), eq(familleThread.id, thread.id)),
    );

  if (input.auteurCote === "staff") {
    await createFamilleNotif({
      etablissementId,
      foyerId: thread.foyerId,
      threadId: thread.id,
      kind: "reply_staff",
      titre: thread.sujet,
      preview: corps || attRows.map((a) => a.fileName).join(", "),
    });
  }

  return { message, attachments: attRows };
}

/** Marque lu les messages de l’autre côté + notifs du thread. */
export async function markFamilleThreadRead(
  etablissementId: string,
  threadId: string,
  readerCote: "staff" | "parent",
) {
  const other = readerCote === "staff" ? "parent" : "staff";
  const db = getDb();
  const now = new Date();
  await db
    .update(familleThreadMessage)
    .set({ luAt: now })
    .where(
      and(
        eq(familleThreadMessage.etablissementId, etablissementId),
        eq(familleThreadMessage.threadId, threadId),
        eq(familleThreadMessage.auteurCote, other),
        sql`${familleThreadMessage.luAt} is null`,
      ),
    );
  if (readerCote === "parent") {
    await db
      .update(familleNotif)
      .set({ luAt: now })
      .where(
        and(
          eq(familleNotif.etablissementId, etablissementId),
          eq(familleNotif.threadId, threadId),
          isNull(familleNotif.luAt),
        ),
      );
  }
}

export async function listFamilleNotifsForFoyers(
  etablissementId: string,
  foyerIds: string[],
  opts?: { unreadOnly?: boolean; limit?: number },
) {
  if (!foyerIds.length) return [];
  const db = getDb();
  const conditions = [
    eq(familleNotif.etablissementId, etablissementId),
    inArray(familleNotif.foyerId, foyerIds),
  ];
  if (opts?.unreadOnly) conditions.push(isNull(familleNotif.luAt));
  return db
    .select()
    .from(familleNotif)
    .where(and(...conditions))
    .orderBy(desc(familleNotif.createdAt))
    .limit(opts?.limit ?? 50);
}

export async function countUnreadNotifsForFoyers(
  etablissementId: string,
  foyerIds: string[],
): Promise<number> {
  if (!foyerIds.length) return 0;
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(familleNotif)
    .where(
      and(
        eq(familleNotif.etablissementId, etablissementId),
        inArray(familleNotif.foyerId, foyerIds),
        isNull(familleNotif.luAt),
      ),
    );
  return Number(row?.n || 0);
}

export async function countUnreadForFoyers(
  etablissementId: string,
  foyerIds: string[],
  readerCote: "staff" | "parent",
): Promise<number> {
  if (!foyerIds.length) return 0;
  const other = readerCote === "staff" ? "parent" : "staff";
  const db = getDb();
  const threads = await db
    .select({ id: familleThread.id })
    .from(familleThread)
    .where(
      and(
        eq(familleThread.etablissementId, etablissementId),
        inArray(familleThread.foyerId, foyerIds),
      ),
    );
  const ids = threads.map((t) => t.id);
  if (!ids.length) return 0;
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(familleThreadMessage)
    .where(
      and(
        eq(familleThreadMessage.etablissementId, etablissementId),
        inArray(familleThreadMessage.threadId, ids),
        eq(familleThreadMessage.auteurCote, other),
        sql`${familleThreadMessage.luAt} is null`,
      ),
    );
  return Number(row?.n || 0);
}

export async function getAttachmentById(
  etablissementId: string,
  attachmentId: string,
): Promise<FamilleAttachmentRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(familleThreadAttachment)
    .where(
      and(
        eq(familleThreadAttachment.etablissementId, etablissementId),
        eq(familleThreadAttachment.id, attachmentId),
      ),
    )
    .limit(1);
  return row ?? null;
}
