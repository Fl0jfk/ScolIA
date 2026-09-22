import "server-only";

import { and, asc, count, eq, isNull, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  internatAffectation,
  internatBatiment,
  internatChambre,
} from "@/db/schema";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import type {
  InternatAffectationRow,
  InternatBatimentRow,
  InternatChambreRow,
} from "@/app/lib/internat-chambres-shared";

function dayBeforeIso(iso: string): string {
  const [y, m, d] = iso.slice(0, 10).split("-").map(Number) as [number, number, number];
  const dt = new Date(Date.UTC(y, m - 1, d));
  dt.setUTCDate(dt.getUTCDate() - 1);
  const yy = dt.getUTCFullYear();
  const mm = String(dt.getUTCMonth() + 1).padStart(2, "0");
  const dd = String(dt.getUTCDate()).padStart(2, "0");
  return `${yy}-${mm}-${dd}`;
}

export async function listInternatBatiments(
  etablissementId: string,
): Promise<InternatBatimentRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: internatBatiment.id,
      label: internatBatiment.label,
      notes: internatBatiment.notes,
    })
    .from(internatBatiment)
    .where(eq(internatBatiment.etablissementId, etablissementId))
    .orderBy(asc(internatBatiment.label));
  return rows;
}

export async function ensureInternatBatiment(
  etablissementId: string,
  label: string,
): Promise<InternatBatimentRow> {
  const db = getDb();
  const trimmed = label.trim();
  if (!trimmed) throw new Error("Libellé bâtiment requis.");
  const [existing] = await db
    .select({
      id: internatBatiment.id,
      label: internatBatiment.label,
      notes: internatBatiment.notes,
    })
    .from(internatBatiment)
    .where(
      and(
        eq(internatBatiment.etablissementId, etablissementId),
        eq(internatBatiment.label, trimmed),
      ),
    )
    .limit(1);
  if (existing) return existing;
  const [row] = await db
    .insert(internatBatiment)
    .values({ etablissementId, label: trimmed })
    .returning({
      id: internatBatiment.id,
      label: internatBatiment.label,
      notes: internatBatiment.notes,
    });
  if (!row) throw new Error("Création bâtiment impossible.");
  return row;
}

export async function createInternatChambre(
  etablissementId: string,
  opts: {
    batimentId?: string;
    batimentLabel?: string;
    label: string;
    capacite?: number;
    etage?: string | null;
    aile?: string | null;
  },
): Promise<InternatChambreRow> {
  const db = getDb();
  const label = opts.label.trim();
  if (!label) throw new Error("Libellé chambre requis.");
  const capacite = Math.min(8, Math.max(1, Number(opts.capacite ?? 2) || 2));

  let batimentId = opts.batimentId?.trim() || "";
  let batimentLabel = "";
  if (batimentId) {
    const [b] = await db
      .select({ id: internatBatiment.id, label: internatBatiment.label })
      .from(internatBatiment)
      .where(
        and(
          eq(internatBatiment.etablissementId, etablissementId),
          eq(internatBatiment.id, batimentId),
        ),
      )
      .limit(1);
    if (!b) throw new Error("Bâtiment introuvable.");
    batimentLabel = b.label;
  } else {
    const b = await ensureInternatBatiment(
      etablissementId,
      opts.batimentLabel?.trim() || "Internat",
    );
    batimentId = b.id;
    batimentLabel = b.label;
  }

  const [row] = await db
    .insert(internatChambre)
    .values({
      etablissementId,
      batimentId,
      label,
      capacite,
      etage: opts.etage?.trim() || null,
      aile: opts.aile?.trim() || null,
    })
    .returning({
      id: internatChambre.id,
      batimentId: internatChambre.batimentId,
      label: internatChambre.label,
      etage: internatChambre.etage,
      capacite: internatChambre.capacite,
      aile: internatChambre.aile,
    });
  if (!row) throw new Error("Création chambre impossible.");
  return {
    ...row,
    batimentLabel,
    placesOccupees: 0,
  };
}

export async function listInternatChambres(
  etablissementId: string,
): Promise<InternatChambreRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: internatChambre.id,
      batimentId: internatChambre.batimentId,
      batimentLabel: internatBatiment.label,
      label: internatChambre.label,
      etage: internatChambre.etage,
      capacite: internatChambre.capacite,
      aile: internatChambre.aile,
      placesOccupees: sql<number>`coalesce((
        select count(*)::int from ${internatAffectation} a
        where a.etablissement_id = ${etablissementId}
          and a.chambre_id = ${internatChambre.id}
          and a.date_fin is null
      ), 0)`,
    })
    .from(internatChambre)
    .innerJoin(
      internatBatiment,
      and(
        eq(internatBatiment.id, internatChambre.batimentId),
        eq(internatBatiment.etablissementId, etablissementId),
      ),
    )
    .where(eq(internatChambre.etablissementId, etablissementId))
    .orderBy(asc(internatBatiment.label), asc(internatChambre.label));

  return rows.map((r) => ({
    ...r,
    placesOccupees: Number(r.placesOccupees) || 0,
  }));
}

export async function listInternatAffectationsOuvertes(
  etablissementId: string,
): Promise<InternatAffectationRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: internatAffectation.id,
      chambreId: internatAffectation.chambreId,
      chambreLabel: internatChambre.label,
      batimentLabel: internatBatiment.label,
      eleveId: internatAffectation.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateDebut: internatAffectation.dateDebut,
      dateFin: internatAffectation.dateFin,
    })
    .from(internatAffectation)
    .innerJoin(
      internatChambre,
      and(
        eq(internatChambre.id, internatAffectation.chambreId),
        eq(internatChambre.etablissementId, etablissementId),
      ),
    )
    .innerJoin(
      internatBatiment,
      and(
        eq(internatBatiment.id, internatChambre.batimentId),
        eq(internatBatiment.etablissementId, etablissementId),
      ),
    )
    .innerJoin(
      eleve,
      and(eq(eleve.id, internatAffectation.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(
      and(
        eq(internatAffectation.etablissementId, etablissementId),
        isNull(internatAffectation.dateFin),
      ),
    )
    .orderBy(asc(internatBatiment.label), asc(internatChambre.label), asc(eleve.nom));

  return rows.map((r) => ({
    ...r,
    dateDebut: String(r.dateDebut).slice(0, 10),
    dateFin: r.dateFin ? String(r.dateFin).slice(0, 10) : null,
  }));
}

/**
 * Ouvre une affectation lit. Ferme l’éventuelle affectation ouverte la veille.
 * Insert / update unitaire — pas de DELETE.
 */
export async function openInternatAffectation(
  etablissementId: string,
  opts: { chambreId: string; eleveId: string; dateDebut?: string },
): Promise<InternatAffectationRow> {
  const db = getDb();
  const dateDebut = (opts.dateDebut ?? calendarDateKeyParis()).slice(0, 10);
  const chambreId = opts.chambreId.trim();
  const eleveId = opts.eleveId.trim();
  if (!chambreId || !eleveId) throw new Error("Chambre et élève requis.");

  const [chambre] = await db
    .select({
      id: internatChambre.id,
      capacite: internatChambre.capacite,
      label: internatChambre.label,
      batimentLabel: internatBatiment.label,
    })
    .from(internatChambre)
    .innerJoin(
      internatBatiment,
      and(
        eq(internatBatiment.id, internatChambre.batimentId),
        eq(internatBatiment.etablissementId, etablissementId),
      ),
    )
    .where(
      and(
        eq(internatChambre.etablissementId, etablissementId),
        eq(internatChambre.id, chambreId),
      ),
    )
    .limit(1);
  if (!chambre) throw new Error("Chambre introuvable.");

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

  const [occ] = await db
    .select({ n: count() })
    .from(internatAffectation)
    .where(
      and(
        eq(internatAffectation.etablissementId, etablissementId),
        eq(internatAffectation.chambreId, chambreId),
        isNull(internatAffectation.dateFin),
      ),
    );
  if (Number(occ?.n ?? 0) >= chambre.capacite) {
    throw new Error(`Chambre ${chambre.label} pleine (${chambre.capacite} place(s)).`);
  }

  const [open] = await db
    .select({ id: internatAffectation.id, chambreId: internatAffectation.chambreId })
    .from(internatAffectation)
    .where(
      and(
        eq(internatAffectation.etablissementId, etablissementId),
        eq(internatAffectation.eleveId, eleveId),
        isNull(internatAffectation.dateFin),
      ),
    )
    .limit(1);

  if (open) {
    if (open.chambreId === chambreId) {
      throw new Error("Cet élève est déjà affecté à cette chambre.");
    }
    await db
      .update(internatAffectation)
      .set({ dateFin: dayBeforeIso(dateDebut), updatedAt: new Date() })
      .where(
        and(
          eq(internatAffectation.etablissementId, etablissementId),
          eq(internatAffectation.id, open.id),
        ),
      );
  }

  const [row] = await db
    .insert(internatAffectation)
    .values({
      etablissementId,
      chambreId,
      eleveId,
      dateDebut,
    })
    .returning({
      id: internatAffectation.id,
      chambreId: internatAffectation.chambreId,
      eleveId: internatAffectation.eleveId,
      dateDebut: internatAffectation.dateDebut,
      dateFin: internatAffectation.dateFin,
    });
  if (!row) throw new Error("Affectation impossible.");

  return {
    id: row.id,
    chambreId: row.chambreId,
    chambreLabel: chambre.label,
    batimentLabel: chambre.batimentLabel,
    eleveId: eleveRow.id,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
    dateDebut: String(row.dateDebut).slice(0, 10),
    dateFin: row.dateFin ? String(row.dateFin).slice(0, 10) : null,
  };
}

/** Ferme une affectation ouverte (date_fin = aujourd’hui ou date fournie). */
export async function closeInternatAffectation(
  etablissementId: string,
  opts: { affectationId: string; dateFin?: string },
): Promise<void> {
  const db = getDb();
  const dateFin = (opts.dateFin ?? calendarDateKeyParis()).slice(0, 10);
  const [updated] = await db
    .update(internatAffectation)
    .set({ dateFin, updatedAt: new Date() })
    .where(
      and(
        eq(internatAffectation.etablissementId, etablissementId),
        eq(internatAffectation.id, opts.affectationId),
        isNull(internatAffectation.dateFin),
      ),
    )
    .returning({ id: internatAffectation.id });
  if (!updated) throw new Error("Affectation ouverte introuvable.");
}
