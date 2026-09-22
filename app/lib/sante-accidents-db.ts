import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, santeAccident } from "@/db/schema";
import type { SanteAccidentRow } from "@/app/lib/sante-accidents-shared";

export type { SanteAccidentRow } from "@/app/lib/sante-accidents-shared";

function dateOnly(d: Date | string): string {
  if (typeof d === "string") return d.slice(0, 10);
  return d.toISOString().slice(0, 10);
}

export async function listSanteAccidents(
  etablissementId: string,
  opts?: { eleveId?: string; limit?: number },
): Promise<SanteAccidentRow[]> {
  const db = getDb();
  const conditions = [eq(santeAccident.etablissementId, etablissementId)];
  if (opts?.eleveId) {
    conditions.push(eq(santeAccident.eleveId, opts.eleveId));
  }

  const rows = await db
    .select({
      id: santeAccident.id,
      eleveId: santeAccident.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateAccident: santeAccident.dateAccident,
      circonstances: santeAccident.circonstances,
      soins: santeAccident.soins,
      suite: santeAccident.suite,
      lieu: santeAccident.lieu,
      auteurNom: santeAccident.auteurNom,
    })
    .from(santeAccident)
    .innerJoin(
      eleve,
      and(eq(eleve.id, santeAccident.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(santeAccident.dateAccident), desc(santeAccident.createdAt))
    .limit(opts?.limit ?? 100);

  return rows.map((r) => ({
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    dateAccident: dateOnly(r.dateAccident),
    circonstances: r.circonstances,
    soins: r.soins,
    suite: r.suite,
    lieu: r.lieu,
    auteurNom: r.auteurNom,
  }));
}

/** Insert unitaire. Pas de DELETE — conservation longue. */
export async function createSanteAccident(
  etablissementId: string,
  opts: {
    eleveId: string;
    dateAccident: string;
    circonstances: string;
    soins?: string;
    suite?: string;
    lieu?: string | null;
    auteurUserId?: string | null;
    auteurNom?: string | null;
  },
): Promise<SanteAccidentRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  const circonstances = opts.circonstances.trim();
  const dateAccident = opts.dateAccident.trim().slice(0, 10);
  if (!eleveId) throw new Error("Élève requis.");
  if (!dateAccident || !/^\d{4}-\d{2}-\d{2}$/.test(dateAccident)) {
    throw new Error("Date d’accident invalide (AAAA-MM-JJ).");
  }
  if (!circonstances) throw new Error("Circonstances requises.");

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
    .insert(santeAccident)
    .values({
      etablissementId,
      eleveId,
      dateAccident,
      circonstances,
      soins: (opts.soins ?? "").trim(),
      suite: (opts.suite ?? "").trim(),
      lieu: (opts.lieu ?? "").trim() || null,
      auteurUserId: opts.auteurUserId ?? null,
      auteurNom: opts.auteurNom ?? null,
    })
    .returning({
      id: santeAccident.id,
      eleveId: santeAccident.eleveId,
      dateAccident: santeAccident.dateAccident,
      circonstances: santeAccident.circonstances,
      soins: santeAccident.soins,
      suite: santeAccident.suite,
      lieu: santeAccident.lieu,
      auteurNom: santeAccident.auteurNom,
    });

  return {
    id: row!.id,
    eleveId: row!.eleveId,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
    dateAccident: dateOnly(row!.dateAccident),
    circonstances: row!.circonstances,
    soins: row!.soins,
    suite: row!.suite,
    lieu: row!.lieu,
    auteurNom: row!.auteurNom,
  };
}
