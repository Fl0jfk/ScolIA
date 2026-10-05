import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, infirmerieFiche } from "@/db/schema";
import type { InfirmerieFicheRow } from "@/app/lib/infirmerie-fiche-shared";

export type { InfirmerieFicheRow } from "@/app/lib/infirmerie-fiche-shared";

function toIso(d: Date | string): string {
  if (typeof d === "string") return d;
  return d.toISOString();
}

export async function getInfirmerieFiche(
  etablissementId: string,
  eleveId: string,
): Promise<InfirmerieFicheRow | null> {
  const db = getDb();
  const id = eleveId.trim();
  if (!id) return null;

  const [eleveRow] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, id)))
    .limit(1);
  if (!eleveRow) return null;

  const [fiche] = await db
    .select({
      id: infirmerieFiche.id,
      eleveId: infirmerieFiche.eleveId,
      antecedents: infirmerieFiche.antecedents,
      personnesAPrevenir: infirmerieFiche.personnesAPrevenir,
      notes: infirmerieFiche.notes,
      updatedAt: infirmerieFiche.updatedAt,
    })
    .from(infirmerieFiche)
    .where(
      and(
        eq(infirmerieFiche.etablissementId, etablissementId),
        eq(infirmerieFiche.eleveId, id),
      ),
    )
    .limit(1);

  if (!fiche) {
    return {
      id: "",
      eleveId: eleveRow.id,
      eleveNom: eleveRow.nom,
      elevePrenom: eleveRow.prenom,
      eleveClasse: eleveRow.classe,
      antecedents: "",
      personnesAPrevenir: "",
      notes: "",
      updatedAt: "",
    };
  }

  return {
    id: fiche.id,
    eleveId: fiche.eleveId,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
    antecedents: fiche.antecedents,
    personnesAPrevenir: fiche.personnesAPrevenir,
    notes: fiche.notes,
    updatedAt: toIso(fiche.updatedAt),
  };
}

/** Upsert unitaire par (etablissement_id, eleve_id). Pas de DELETE. */
export async function upsertInfirmerieFiche(
  etablissementId: string,
  opts: {
    eleveId: string;
    antecedents?: string;
    personnesAPrevenir?: string;
    notes?: string;
  },
): Promise<InfirmerieFicheRow> {
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

  const antecedents = (opts.antecedents ?? "").trim();
  const personnesAPrevenir = (opts.personnesAPrevenir ?? "").trim();
  const notes = (opts.notes ?? "").trim();
  const now = new Date();

  const [row] = await db
    .insert(infirmerieFiche)
    .values({
      etablissementId,
      eleveId,
      antecedents,
      personnesAPrevenir,
      notes,
      updatedAt: now,
      createdAt: now,
    })
    .onConflictDoUpdate({
      target: [infirmerieFiche.etablissementId, infirmerieFiche.eleveId],
      set: {
        antecedents,
        personnesAPrevenir,
        notes,
        updatedAt: now,
      },
    })
    .returning({
      id: infirmerieFiche.id,
      eleveId: infirmerieFiche.eleveId,
      antecedents: infirmerieFiche.antecedents,
      personnesAPrevenir: infirmerieFiche.personnesAPrevenir,
      notes: infirmerieFiche.notes,
      updatedAt: infirmerieFiche.updatedAt,
    });

  return {
    id: row!.id,
    eleveId: row!.eleveId,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
    antecedents: row!.antecedents,
    personnesAPrevenir: row!.personnesAPrevenir,
    notes: row!.notes,
    updatedAt: toIso(row!.updatedAt),
  };
}
