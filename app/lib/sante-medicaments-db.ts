import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, santeMedicamentPrise } from "@/db/schema";
import type { SanteMedicamentPriseRow } from "@/app/lib/sante-medicaments-shared";

export type { SanteMedicamentPriseRow } from "@/app/lib/sante-medicaments-shared";

function toIso(d: Date | string): string {
  if (typeof d === "string") return d;
  return d.toISOString();
}

export async function listSanteMedicamentPrises(
  etablissementId: string,
  opts?: { eleveId?: string; limit?: number },
): Promise<SanteMedicamentPriseRow[]> {
  const db = getDb();
  const conditions = [eq(santeMedicamentPrise.etablissementId, etablissementId)];
  if (opts?.eleveId) {
    conditions.push(eq(santeMedicamentPrise.eleveId, opts.eleveId));
  }

  const rows = await db
    .select({
      id: santeMedicamentPrise.id,
      eleveId: santeMedicamentPrise.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      medicament: santeMedicamentPrise.medicament,
      dose: santeMedicamentPrise.dose,
      prisAt: santeMedicamentPrise.prisAt,
      auteurNom: santeMedicamentPrise.auteurNom,
      notes: santeMedicamentPrise.notes,
    })
    .from(santeMedicamentPrise)
    .innerJoin(
      eleve,
      and(eq(eleve.id, santeMedicamentPrise.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(santeMedicamentPrise.prisAt))
    .limit(opts?.limit ?? 100);

  return rows.map((r) => ({
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    medicament: r.medicament,
    dose: r.dose,
    prisAt: toIso(r.prisAt),
    auteurNom: r.auteurNom,
    notes: r.notes,
  }));
}

/** Insert unitaire. Pas de DELETE. */
export async function createSanteMedicamentPrise(
  etablissementId: string,
  opts: {
    eleveId: string;
    medicament: string;
    dose?: string | null;
    prisAt?: string | null;
    notes?: string | null;
    auteurUserId?: string | null;
    auteurNom?: string | null;
  },
): Promise<SanteMedicamentPriseRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  const medicament = opts.medicament.trim();
  if (!eleveId) throw new Error("Élève requis.");
  if (!medicament) throw new Error("Médicament requis.");

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

  let prisAt = new Date();
  if (opts.prisAt?.trim()) {
    const parsed = new Date(opts.prisAt.trim());
    if (Number.isNaN(parsed.getTime())) throw new Error("Date/heure invalide.");
    prisAt = parsed;
  }

  const [row] = await db
    .insert(santeMedicamentPrise)
    .values({
      etablissementId,
      eleveId,
      medicament,
      dose: (opts.dose ?? "").trim() || null,
      prisAt,
      notes: (opts.notes ?? "").trim() || null,
      auteurUserId: opts.auteurUserId ?? null,
      auteurNom: opts.auteurNom ?? null,
    })
    .returning({
      id: santeMedicamentPrise.id,
      eleveId: santeMedicamentPrise.eleveId,
      medicament: santeMedicamentPrise.medicament,
      dose: santeMedicamentPrise.dose,
      prisAt: santeMedicamentPrise.prisAt,
      auteurNom: santeMedicamentPrise.auteurNom,
      notes: santeMedicamentPrise.notes,
    });

  return {
    id: row!.id,
    eleveId: row!.eleveId,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
    medicament: row!.medicament,
    dose: row!.dose,
    prisAt: toIso(row!.prisAt),
    auteurNom: row!.auteurNom,
    notes: row!.notes,
  };
}
