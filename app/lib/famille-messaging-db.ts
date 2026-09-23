import "server-only";

import { and, asc, desc, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  familleThread,
  familleThreadMessage,
  foyer,
} from "@/db/schema";

export type FamilleThreadRow = typeof familleThread.$inferSelect;
export type FamilleThreadMessageRow = typeof familleThreadMessage.$inferSelect;

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

export async function listStaffFamilleThreads(etablissementId: string) {
  const db = getDb();
  return db
    .select({
      id: familleThread.id,
      foyerId: familleThread.foyerId,
      eleveId: familleThread.eleveId,
      sujet: familleThread.sujet,
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

export async function createFamilleThreadWithMessage(
  etablissementId: string,
  input: {
    foyerId: string;
    eleveId?: string | null;
    sujet: string;
    corps: string;
    auteurUserId: string;
    auteurNom: string;
  },
) {
  const sujet = input.sujet.trim().slice(0, 200);
  const corps = trimCorps(input.corps);
  if (!sujet) throw new Error("Sujet obligatoire.");
  if (!corps) throw new Error("Message obligatoire.");
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
      corps,
      createdAt: now,
    })
    .returning();
  if (!message) throw new Error("Création message impossible.");
  return { thread, message };
}

export async function replyFamilleThreadMessage(
  etablissementId: string,
  input: {
    threadId: string;
    auteurCote: "staff" | "parent";
    auteurUserId: string;
    auteurNom: string;
    corps: string;
  },
) {
  const corps = trimCorps(input.corps);
  if (!corps) throw new Error("Message obligatoire.");
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
      corps,
      createdAt: now,
    })
    .returning();
  if (!message) throw new Error("Envoi impossible.");

  await db
    .update(familleThread)
    .set({ lastMessageAt: now, updatedAt: now })
    .where(
      and(eq(familleThread.etablissementId, etablissementId), eq(familleThread.id, thread.id)),
    );

  return message;
}

/** Marque lu les messages de l’autre côté (parent lit staff, staff lit parent). */
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
