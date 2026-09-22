import "server-only";

import { and, desc, eq, ne } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveDocument, santePai } from "@/db/schema";
import { detectAccompagnementKind } from "@/app/lib/eleve-pap";
import {
  isSantePaiStatut,
  type SantePaiDocumentLite,
  type SantePaiRow,
  type SantePaiStatut,
} from "@/app/lib/sante-pai-shared";

export {
  SANTE_PAI_STATUTS,
  SANTE_PAI_STATUT_LABELS,
  isSantePaiStatut,
  type SantePaiDocumentLite,
  type SantePaiRow,
  type SantePaiStatut,
} from "@/app/lib/sante-pai-shared";

function toIso(d: Date | string | null | undefined): string | null {
  if (!d) return null;
  if (typeof d === "string") return d;
  return d.toISOString();
}

function dateOnly(d: Date | string | null | undefined): string | null {
  if (!d) return null;
  if (typeof d === "string") return d.slice(0, 10);
  return d.toISOString().slice(0, 10);
}

async function mapPai(
  etablissementId: string,
  r: {
    id: string;
    eleveId: string;
    eleveNom: string;
    elevePrenom: string;
    eleveClasse: string | null;
    statut: string;
    protocole: string;
    traitementsAutorises: string;
    documentId: string | null;
    dateDebut: string | Date | null;
    dateFin: string | Date | null;
    valideAt: Date | string | null;
    valideParNom: string | null;
    notes: string;
    updatedAt: Date | string;
  },
): Promise<SantePaiRow> {
  let documentTitle: string | null = null;
  if (r.documentId) {
    const db = getDb();
    const [doc] = await db
      .select({ title: eleveDocument.title })
      .from(eleveDocument)
      .where(
        and(
          eq(eleveDocument.etablissementId, etablissementId),
          eq(eleveDocument.id, r.documentId),
        ),
      )
      .limit(1);
    documentTitle = doc?.title ?? null;
  }
  return {
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    statut: isSantePaiStatut(r.statut) ? r.statut : "brouillon",
    protocole: r.protocole,
    traitementsAutorises: r.traitementsAutorises,
    documentId: r.documentId,
    documentTitle,
    dateDebut: dateOnly(r.dateDebut),
    dateFin: dateOnly(r.dateFin),
    valideAt: toIso(r.valideAt),
    valideParNom: r.valideParNom,
    notes: r.notes,
    updatedAt: toIso(r.updatedAt) ?? "",
  };
}

export async function listSantePai(
  etablissementId: string,
  opts?: { eleveId?: string; statut?: string },
): Promise<SantePaiRow[]> {
  const db = getDb();
  const conditions = [eq(santePai.etablissementId, etablissementId)];
  if (opts?.eleveId) conditions.push(eq(santePai.eleveId, opts.eleveId));
  if (opts?.statut && isSantePaiStatut(opts.statut)) {
    conditions.push(eq(santePai.statut, opts.statut));
  }

  const rows = await db
    .select({
      id: santePai.id,
      eleveId: santePai.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      statut: santePai.statut,
      protocole: santePai.protocole,
      traitementsAutorises: santePai.traitementsAutorises,
      documentId: santePai.documentId,
      dateDebut: santePai.dateDebut,
      dateFin: santePai.dateFin,
      valideAt: santePai.valideAt,
      valideParNom: santePai.valideParNom,
      notes: santePai.notes,
      updatedAt: santePai.updatedAt,
    })
    .from(santePai)
    .innerJoin(
      eleve,
      and(eq(eleve.id, santePai.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(santePai.updatedAt))
    .limit(100);

  return Promise.all(rows.map((r) => mapPai(etablissementId, r)));
}

export async function getSantePai(
  etablissementId: string,
  id: string,
): Promise<SantePaiRow | null> {
  const db = getDb();
  const [row] = await db
    .select({
      id: santePai.id,
      eleveId: santePai.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      statut: santePai.statut,
      protocole: santePai.protocole,
      traitementsAutorises: santePai.traitementsAutorises,
      documentId: santePai.documentId,
      dateDebut: santePai.dateDebut,
      dateFin: santePai.dateFin,
      valideAt: santePai.valideAt,
      valideParNom: santePai.valideParNom,
      notes: santePai.notes,
      updatedAt: santePai.updatedAt,
    })
    .from(santePai)
    .innerJoin(
      eleve,
      and(eq(eleve.id, santePai.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(eq(santePai.etablissementId, etablissementId), eq(santePai.id, id)))
    .limit(1);
  if (!row) return null;
  return mapPai(etablissementId, row);
}

/** Documents tiroir santé susceptibles d’être un PAI (titre). */
export async function listDocumentsPaiCandidats(
  etablissementId: string,
  eleveId: string,
): Promise<SantePaiDocumentLite[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: eleveDocument.id,
      title: eleveDocument.title,
      createdAt: eleveDocument.createdAt,
    })
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, etablissementId),
        eq(eleveDocument.eleveId, eleveId),
        eq(eleveDocument.tiroir, "sante"),
      ),
    )
    .orderBy(desc(eleveDocument.createdAt))
    .limit(50);

  return rows
    .filter((r) => detectAccompagnementKind(r.title) === "pai" || /pai/i.test(r.title))
    .map((r) => ({
      id: r.id,
      title: r.title,
      createdAt: toIso(r.createdAt) ?? "",
    }));
}

/**
 * Crée une entrée document « PAI » sans fichier (référence dossier).
 * Utile quand le PDF n’est pas encore uploadé.
 */
export async function ensurePaiDocumentPlaceholder(
  etablissementId: string,
  eleveId: string,
  opts?: { anneeLabel?: string | null; createdByUserId?: string | null },
): Promise<SantePaiDocumentLite> {
  const db = getDb();
  const annee = (opts?.anneeLabel ?? "").trim() || new Date().getFullYear().toString();
  const title = `PAI ${annee}`;

  const existing = await listDocumentsPaiCandidats(etablissementId, eleveId);
  const hit = existing.find((d) => d.title === title);
  if (hit) return hit;

  const [eleveRow] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) throw new Error("Élève introuvable.");

  const [doc] = await db
    .insert(eleveDocument)
    .values({
      etablissementId,
      eleveId,
      tiroir: "sante",
      title,
      anneeLabel: annee,
      confidentialite: "sante",
      source: "infirmerie",
      createdByUserId: opts?.createdByUserId ?? null,
    })
    .returning({
      id: eleveDocument.id,
      title: eleveDocument.title,
      createdAt: eleveDocument.createdAt,
    });

  return {
    id: doc!.id,
    title: doc!.title,
    createdAt: toIso(doc!.createdAt) ?? "",
  };
}

export async function createSantePai(
  etablissementId: string,
  opts: {
    eleveId: string;
    protocole?: string;
    traitementsAutorises?: string;
    documentId?: string | null;
    dateDebut?: string | null;
    dateFin?: string | null;
    notes?: string;
  },
): Promise<SantePaiRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  if (!eleveId) throw new Error("Élève requis.");

  const [eleveRow] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) throw new Error("Élève introuvable.");

  let documentId = opts.documentId?.trim() || null;
  if (documentId) {
    const [doc] = await db
      .select({ id: eleveDocument.id })
      .from(eleveDocument)
      .where(
        and(
          eq(eleveDocument.etablissementId, etablissementId),
          eq(eleveDocument.eleveId, eleveId),
          eq(eleveDocument.id, documentId),
        ),
      )
      .limit(1);
    if (!doc) throw new Error("Document introuvable pour cet élève.");
  }

  const [row] = await db
    .insert(santePai)
    .values({
      etablissementId,
      eleveId,
      statut: "brouillon",
      protocole: (opts.protocole ?? "").trim(),
      traitementsAutorises: (opts.traitementsAutorises ?? "").trim(),
      documentId,
      dateDebut: opts.dateDebut?.trim()?.slice(0, 10) || null,
      dateFin: opts.dateFin?.trim()?.slice(0, 10) || null,
      notes: (opts.notes ?? "").trim(),
    })
    .returning({
      id: santePai.id,
      eleveId: santePai.eleveId,
      statut: santePai.statut,
      protocole: santePai.protocole,
      traitementsAutorises: santePai.traitementsAutorises,
      documentId: santePai.documentId,
      dateDebut: santePai.dateDebut,
      dateFin: santePai.dateFin,
      valideAt: santePai.valideAt,
      valideParNom: santePai.valideParNom,
      notes: santePai.notes,
      updatedAt: santePai.updatedAt,
    });

  return mapPai(etablissementId, {
    ...row!,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
  });
}

/** Upsert champs métier par id + etablissement_id. Pas de DELETE. */
export async function updateSantePai(
  etablissementId: string,
  id: string,
  opts: {
    protocole?: string;
    traitementsAutorises?: string;
    documentId?: string | null;
    dateDebut?: string | null;
    dateFin?: string | null;
    notes?: string;
  },
): Promise<SantePaiRow> {
  const db = getDb();
  const existing = await getSantePai(etablissementId, id);
  if (!existing) throw new Error("PAI introuvable.");
  if (existing.statut === "revoque") throw new Error("PAI révoqué — créer un nouveau brouillon.");

  let documentId =
    opts.documentId === undefined ? existing.documentId : opts.documentId?.trim() || null;
  if (documentId) {
    const [doc] = await db
      .select({ id: eleveDocument.id })
      .from(eleveDocument)
      .where(
        and(
          eq(eleveDocument.etablissementId, etablissementId),
          eq(eleveDocument.eleveId, existing.eleveId),
          eq(eleveDocument.id, documentId),
        ),
      )
      .limit(1);
    if (!doc) throw new Error("Document introuvable pour cet élève.");
  }

  const now = new Date();
  await db
    .update(santePai)
    .set({
      protocole:
        opts.protocole !== undefined ? opts.protocole.trim() : existing.protocole,
      traitementsAutorises:
        opts.traitementsAutorises !== undefined
          ? opts.traitementsAutorises.trim()
          : existing.traitementsAutorises,
      documentId,
      dateDebut:
        opts.dateDebut !== undefined
          ? opts.dateDebut?.trim()?.slice(0, 10) || null
          : existing.dateDebut,
      dateFin:
        opts.dateFin !== undefined
          ? opts.dateFin?.trim()?.slice(0, 10) || null
          : existing.dateFin,
      notes: opts.notes !== undefined ? opts.notes.trim() : existing.notes,
      updatedAt: now,
    })
    .where(and(eq(santePai.etablissementId, etablissementId), eq(santePai.id, id)));

  const updated = await getSantePai(etablissementId, id);
  if (!updated) throw new Error("PAI introuvable après mise à jour.");
  return updated;
}

/**
 * Valide le PAI. Soft-révoque les autres PAI « valide » du même élève.
 * Protocole requis. Pas de DELETE.
 */
export async function validerSantePai(
  etablissementId: string,
  id: string,
  opts: { auteurUserId?: string | null; auteurNom?: string | null },
): Promise<SantePaiRow> {
  const db = getDb();
  const existing = await getSantePai(etablissementId, id);
  if (!existing) throw new Error("PAI introuvable.");
  if (existing.statut === "revoque") throw new Error("PAI révoqué.");
  if (!existing.protocole.trim()) {
    throw new Error("Protocole requis avant validation.");
  }
  if (!existing.traitementsAutorises.trim()) {
    throw new Error("Traitements autorisés requis avant validation.");
  }

  const now = new Date();

  await db
    .update(santePai)
    .set({ statut: "revoque", updatedAt: now })
    .where(
      and(
        eq(santePai.etablissementId, etablissementId),
        eq(santePai.eleveId, existing.eleveId),
        eq(santePai.statut, "valide"),
        ne(santePai.id, id),
      ),
    );

  await db
    .update(santePai)
    .set({
      statut: "valide",
      valideAt: now,
      valideParUserId: opts.auteurUserId ?? null,
      valideParNom: opts.auteurNom ?? null,
      updatedAt: now,
    })
    .where(and(eq(santePai.etablissementId, etablissementId), eq(santePai.id, id)));

  const updated = await getSantePai(etablissementId, id);
  if (!updated) throw new Error("PAI introuvable après validation.");
  return updated;
}

/** Soft : révoque. Pas de DELETE. */
export async function revoquerSantePai(
  etablissementId: string,
  id: string,
): Promise<SantePaiRow> {
  const db = getDb();
  const existing = await getSantePai(etablissementId, id);
  if (!existing) throw new Error("PAI introuvable.");
  const now = new Date();
  await db
    .update(santePai)
    .set({ statut: "revoque", updatedAt: now })
    .where(and(eq(santePai.etablissementId, etablissementId), eq(santePai.id, id)));
  const updated = await getSantePai(etablissementId, id);
  if (!updated) throw new Error("PAI introuvable après révocation.");
  return updated;
}
