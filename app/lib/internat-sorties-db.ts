import "server-only";

import { and, asc, desc, eq, gte, lte } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, internatSortie } from "@/db/schema";
import { listEleveIdsEnSortieOnDate } from "@/app/lib/occupancy/travels-read";
import type { InternatSortieRow } from "@/app/lib/internat-sorties-shared";

function mapRow(r: {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string | Date;
  dateFin: string | Date;
  motif: string | null;
}): InternatSortieRow {
  return {
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    dateDebut: String(r.dateDebut).slice(0, 10),
    dateFin: String(r.dateFin).slice(0, 10),
    motif: r.motif,
  };
}

export async function listInternatSorties(
  etablissementId: string,
  opts?: { from?: string; to?: string; limit?: number },
): Promise<InternatSortieRow[]> {
  const db = getDb();
  const conditions = [eq(internatSortie.etablissementId, etablissementId)];
  if (opts?.from) {
    conditions.push(gte(internatSortie.dateFin, opts.from.slice(0, 10)));
  }
  if (opts?.to) {
    conditions.push(lte(internatSortie.dateDebut, opts.to.slice(0, 10)));
  }

  const rows = await db
    .select({
      id: internatSortie.id,
      eleveId: internatSortie.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateDebut: internatSortie.dateDebut,
      dateFin: internatSortie.dateFin,
      motif: internatSortie.motif,
    })
    .from(internatSortie)
    .innerJoin(
      eleve,
      and(eq(eleve.id, internatSortie.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(internatSortie.dateDebut), asc(eleve.nom))
    .limit(opts?.limit ?? 100);

  return rows.map(mapRow);
}

/** Élèves en sortie sur une date donnée (pour croiser l’appel). */
export async function listEleveIdsEnSortieLeJour(
  etablissementId: string,
  date: string,
): Promise<Set<string>> {
  const db = getDb();
  const day = date.slice(0, 10);
  const rows = await db
    .select({ eleveId: internatSortie.eleveId })
    .from(internatSortie)
    .where(
      and(
        eq(internatSortie.etablissementId, etablissementId),
        lte(internatSortie.dateDebut, day),
        gte(internatSortie.dateFin, day),
      ),
    );
  return new Set(rows.map((r) => r.eleveId));
}

/**
 * Enregistre une sortie week-end / correspondant.
 * Insert unitaire — pas de DELETE.
 */
export async function createInternatSortie(
  etablissementId: string,
  opts: {
    eleveId: string;
    dateDebut: string;
    dateFin: string;
    motif?: string | null;
  },
): Promise<InternatSortieRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  const dateDebut = opts.dateDebut.slice(0, 10);
  const dateFin = opts.dateFin.slice(0, 10);
  if (!eleveId) throw new Error("Élève requis.");
  if (dateFin < dateDebut) throw new Error("La date de fin doit être ≥ début.");

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

  const [row] = await db
    .insert(internatSortie)
    .values({
      etablissementId,
      eleveId,
      dateDebut,
      dateFin,
      motif: opts.motif?.trim() || null,
    })
    .returning({
      id: internatSortie.id,
      eleveId: internatSortie.eleveId,
      dateDebut: internatSortie.dateDebut,
      dateFin: internatSortie.dateFin,
      motif: internatSortie.motif,
    });
  if (!row) throw new Error("Création sortie impossible.");

  return mapRow({
    ...row,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
  });
}

/** Soft-close : raccourcit date_fin à hier (ou date fournie). Pas de hard DELETE. */
export async function closeInternatSortie(
  etablissementId: string,
  opts: { sortieId: string; dateFin?: string },
): Promise<void> {
  const db = getDb();
  const dateFin =
    opts.dateFin?.slice(0, 10) ??
    (() => {
      const d = new Date();
      return d.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
    })();

  const [existing] = await db
    .select({
      id: internatSortie.id,
      dateDebut: internatSortie.dateDebut,
    })
    .from(internatSortie)
    .where(
      and(
        eq(internatSortie.etablissementId, etablissementId),
        eq(internatSortie.id, opts.sortieId),
      ),
    )
    .limit(1);
  if (!existing) throw new Error("Sortie introuvable.");
  const debut = String(existing.dateDebut).slice(0, 10);
  if (dateFin < debut) throw new Error("Date de fin invalide.");

  await db
    .update(internatSortie)
    .set({ dateFin, updatedAt: new Date() })
    .where(
      and(
        eq(internatSortie.etablissementId, etablissementId),
        eq(internatSortie.id, opts.sortieId),
      ),
    );
}

/**
 * Élèves absents de l’internat ce jour-là :
 * sorties week-end (`internat_sortie`) ∪ voyages scolaires (`en_sortie`).
 */
export async function listEleveIdsHorsInternatLeJour(
  etablissementId: string,
  date: string,
): Promise<Set<string>> {
  const [weekend, travels] = await Promise.all([
    listEleveIdsEnSortieLeJour(etablissementId, date),
    listEleveIdsEnSortieOnDate({ etablissementId, date }),
  ]);
  const out = new Set(weekend);
  for (const id of travels) out.add(id);
  return out;
}

/** Compte sorties actives croisant une date (debug / synthèse). */
export async function countSortiesActivesLeJour(
  etablissementId: string,
  date: string,
): Promise<number> {
  const set = await listEleveIdsHorsInternatLeJour(etablissementId, date);
  return set.size;
}
