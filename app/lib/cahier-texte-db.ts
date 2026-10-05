/**
 * Cahier de textes — leçon + travail à faire (distinct des devoirs notés).
 */

import "server-only";

import { and, asc, desc, eq, gte, inArray, lte, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { cahierTexte, eleve } from "@/db/schema";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";

export type CahierTexteRow = typeof cahierTexte.$inferSelect;

export type CahierTexteInput = {
  dateSeance: string;
  classe: string;
  matiereLibelle?: string | null;
  contenu?: string;
  travail?: string;
  aRendreLe?: string | null;
  creneauId?: string | null;
  visibleFamille?: boolean;
  enseignantUserId?: string | null;
  enseignantNom?: string | null;
};

function trimText(v: string | null | undefined, max: number): string {
  return String(v || "")
    .trim()
    .slice(0, max);
}

function requireDate(iso: string, label: string): string {
  const d = String(iso || "").trim().slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(d)) {
    throw new Error(`${label} invalide (AAAA-MM-JJ).`);
  }
  return d;
}

export async function listDistinctClasses(etablissementId: string): Promise<string[]> {
  const db = getDb();
  const rows = await db
    .selectDistinct({ classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), sql`${eleve.classe} is not null`))
    .orderBy(asc(eleve.classe));
  return rows
    .map((r) => String(r.classe || "").trim())
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b, "fr"));
}

export async function listCahierTexte(
  etablissementId: string,
  opts?: {
    classe?: string | null;
    from?: string | null;
    to?: string | null;
    limit?: number;
  },
): Promise<CahierTexteRow[]> {
  const db = getDb();
  const conditions = [eq(cahierTexte.etablissementId, etablissementId)];
  if (opts?.classe?.trim()) {
    conditions.push(eq(cahierTexte.classe, opts.classe.trim()));
  }
  if (opts?.from?.trim()) {
    conditions.push(gte(cahierTexte.dateSeance, requireDate(opts.from, "Date début")));
  }
  if (opts?.to?.trim()) {
    conditions.push(lte(cahierTexte.dateSeance, requireDate(opts.to, "Date fin")));
  }
  const limit = Math.min(Math.max(Number(opts?.limit) || 80, 1), 200);
  return db
    .select()
    .from(cahierTexte)
    .where(and(...conditions))
    .orderBy(desc(cahierTexte.dateSeance), desc(cahierTexte.updatedAt))
    .limit(limit);
}

export async function createCahierTexte(
  etablissementId: string,
  input: CahierTexteInput,
): Promise<CahierTexteRow> {
  const dateSeance = requireDate(input.dateSeance, "Date de séance");
  const classe = trimText(input.classe, 40);
  if (!classe) throw new Error("Classe obligatoire.");
  const contenu = trimText(input.contenu, 8000);
  const travail = trimText(input.travail, 4000);
  if (!contenu && !travail) {
    throw new Error("Renseigner le contenu de la leçon ou le travail à faire.");
  }
  const aRendreLe = input.aRendreLe?.trim()
    ? requireDate(input.aRendreLe, "Date de rendu")
    : null;

  const db = getDb();
  const now = new Date();
  const [row] = await db
    .insert(cahierTexte)
    .values({
      etablissementId,
      dateSeance,
      classe,
      matiereLibelle: trimText(input.matiereLibelle, 120) || null,
      contenu,
      travail,
      aRendreLe,
      creneauId: input.creneauId?.trim() || null,
      visibleFamille: input.visibleFamille !== false,
      enseignantUserId: input.enseignantUserId || null,
      enseignantNom: trimText(input.enseignantNom, 120) || null,
      createdAt: now,
      updatedAt: now,
    })
    .returning();
  if (!row) throw new Error("Création impossible.");
  return row;
}

export async function updateCahierTexte(
  etablissementId: string,
  id: string,
  patch: Partial<CahierTexteInput>,
): Promise<CahierTexteRow> {
  const db = getDb();
  const existing = await db
    .select()
    .from(cahierTexte)
    .where(and(eq(cahierTexte.etablissementId, etablissementId), eq(cahierTexte.id, id)))
    .limit(1)
    .then((r) => r[0] ?? null);
  if (!existing) throw new Error("Entrée introuvable.");

  const next = {
    dateSeance: patch.dateSeance
      ? requireDate(patch.dateSeance, "Date de séance")
      : existing.dateSeance,
    classe: patch.classe !== undefined ? trimText(patch.classe, 40) : existing.classe,
    matiereLibelle:
      patch.matiereLibelle !== undefined
        ? trimText(patch.matiereLibelle, 120) || null
        : existing.matiereLibelle,
    contenu: patch.contenu !== undefined ? trimText(patch.contenu, 8000) : existing.contenu,
    travail: patch.travail !== undefined ? trimText(patch.travail, 4000) : existing.travail,
    aRendreLe:
      patch.aRendreLe !== undefined
        ? patch.aRendreLe?.trim()
          ? requireDate(patch.aRendreLe, "Date de rendu")
          : null
        : existing.aRendreLe,
    visibleFamille:
      patch.visibleFamille !== undefined ? !!patch.visibleFamille : existing.visibleFamille,
    updatedAt: new Date(),
  };
  if (!next.classe) throw new Error("Classe obligatoire.");
  if (!next.contenu && !next.travail) {
    throw new Error("Renseigner le contenu de la leçon ou le travail à faire.");
  }

  const [row] = await db
    .update(cahierTexte)
    .set(next)
    .where(and(eq(cahierTexte.etablissementId, etablissementId), eq(cahierTexte.id, id)))
    .returning();
  if (!row) throw new Error("Mise à jour impossible.");
  return row;
}

/** Entrées visibles famille pour les classes des enfants. */
export async function listCahierTexteForFamille(
  etablissementId: string,
  enfants: Array<{ id: string; classe: string | null; nom: string; prenom: string }>,
  opts?: { from?: string | null; to?: string | null; limit?: number },
): Promise<
  Array<
    CahierTexteRow & {
      enfantsConcernes: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
    }
  >
> {
  const classes = [
    ...new Set(enfants.map((e) => String(e.classe || "").trim()).filter(Boolean)),
  ];
  if (!classes.length) return [];

  const rows = await listCahierTexte(etablissementId, {
    from: opts?.from,
    to: opts?.to,
    limit: opts?.limit ?? 60,
  });

  return rows
    .filter((r) => r.visibleFamille)
    .filter((r) => classes.some((c) => schoolClassesMatch(c, r.classe)))
    .map((r) => ({
      ...r,
      enfantsConcernes: enfants.filter((e) => schoolClassesMatch(e.classe, r.classe)),
    }));
}

/** Soft-hide : retire la visibilité famille (pas de DELETE massif). */
export async function hideCahierTexteFromFamille(
  etablissementId: string,
  id: string,
): Promise<CahierTexteRow> {
  return updateCahierTexte(etablissementId, id, { visibleFamille: false });
}

export async function getCahierTexteByIds(
  etablissementId: string,
  ids: string[],
): Promise<CahierTexteRow[]> {
  if (!ids.length) return [];
  const db = getDb();
  return db
    .select()
    .from(cahierTexte)
    .where(and(eq(cahierTexte.etablissementId, etablissementId), inArray(cahierTexte.id, ids)));
}
