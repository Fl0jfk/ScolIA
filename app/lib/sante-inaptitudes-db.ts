import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, santeExtrait, santeInaptitudeEps } from "@/db/schema";
import { createSanteExtrait, desactiverSanteExtrait } from "@/app/lib/sante-extraits-db";
import type { SanteInaptitudeEpsRow } from "@/app/lib/sante-inaptitudes-shared";

export type { SanteInaptitudeEpsRow } from "@/app/lib/sante-inaptitudes-shared";

function dateOnly(d: Date | string | null | undefined): string | null {
  if (!d) return null;
  if (typeof d === "string") return d.slice(0, 10);
  return d.toISOString().slice(0, 10);
}

function mapRow(r: {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string | Date;
  dateFin: string | Date | null;
  motif: string;
  libelleExtrait: string;
  documentId: string | null;
  extraitId: string | null;
  actif: boolean;
  auteurNom: string | null;
}): SanteInaptitudeEpsRow {
  return {
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    dateDebut: dateOnly(r.dateDebut) ?? "",
    dateFin: dateOnly(r.dateFin),
    motif: r.motif,
    libelleExtrait: r.libelleExtrait,
    documentId: r.documentId,
    extraitId: r.extraitId,
    actif: r.actif,
    auteurNom: r.auteurNom,
  };
}

export async function listSanteInaptitudesEps(
  etablissementId: string,
  opts?: { eleveId?: string; actifsSeulement?: boolean },
): Promise<SanteInaptitudeEpsRow[]> {
  const db = getDb();
  const conditions = [eq(santeInaptitudeEps.etablissementId, etablissementId)];
  if (opts?.eleveId) conditions.push(eq(santeInaptitudeEps.eleveId, opts.eleveId));
  if (opts?.actifsSeulement !== false) {
    conditions.push(eq(santeInaptitudeEps.actif, true));
  }

  const rows = await db
    .select({
      id: santeInaptitudeEps.id,
      eleveId: santeInaptitudeEps.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateDebut: santeInaptitudeEps.dateDebut,
      dateFin: santeInaptitudeEps.dateFin,
      motif: santeInaptitudeEps.motif,
      libelleExtrait: santeInaptitudeEps.libelleExtrait,
      documentId: santeInaptitudeEps.documentId,
      extraitId: santeInaptitudeEps.extraitId,
      actif: santeInaptitudeEps.actif,
      auteurNom: santeInaptitudeEps.auteurNom,
    })
    .from(santeInaptitudeEps)
    .innerJoin(
      eleve,
      and(eq(eleve.id, santeInaptitudeEps.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(santeInaptitudeEps.dateDebut))
    .limit(100);

  return rows.map(mapRow);
}

/**
 * Crée l’inaptitude et un extrait EPS actif lié.
 * Upsert / insert unitaires — pas de DELETE.
 */
export async function createSanteInaptitudeEps(
  etablissementId: string,
  opts: {
    eleveId: string;
    dateDebut: string;
    dateFin?: string | null;
    motif?: string;
    libelleExtrait: string;
    documentId?: string | null;
    auteurUserId?: string | null;
    auteurNom?: string | null;
  },
): Promise<SanteInaptitudeEpsRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  const dateDebut = opts.dateDebut.trim().slice(0, 10);
  const libelleExtrait = opts.libelleExtrait.trim();
  if (!eleveId) throw new Error("Élève requis.");
  if (!dateDebut || !/^\d{4}-\d{2}-\d{2}$/.test(dateDebut)) {
    throw new Error("Date de début invalide (AAAA-MM-JJ).");
  }
  if (!libelleExtrait) throw new Error("Libellé EPS requis (ce que le prof voit).");

  const dateFin = opts.dateFin?.trim()?.slice(0, 10) || null;
  if (dateFin && !/^\d{4}-\d{2}-\d{2}$/.test(dateFin)) {
    throw new Error("Date de fin invalide.");
  }
  if (dateFin && dateFin < dateDebut) {
    throw new Error("La date de fin doit être après le début.");
  }

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

  const periode =
    dateFin && dateFin !== dateDebut
      ? `du ${dateDebut} au ${dateFin}`
      : `à partir du ${dateDebut}`;
  const libelleComplet = `${libelleExtrait} (${periode})`;

  const extrait = await createSanteExtrait(etablissementId, {
    eleveId,
    portee: "eps",
    libelle: libelleComplet,
    documentId: opts.documentId ?? null,
  });

  const [row] = await db
    .insert(santeInaptitudeEps)
    .values({
      etablissementId,
      eleveId,
      dateDebut,
      dateFin,
      motif: (opts.motif ?? "").trim(),
      libelleExtrait,
      documentId: opts.documentId ?? null,
      extraitId: extrait.id,
      actif: true,
      auteurUserId: opts.auteurUserId ?? null,
      auteurNom: opts.auteurNom ?? null,
    })
    .returning({
      id: santeInaptitudeEps.id,
      eleveId: santeInaptitudeEps.eleveId,
      dateDebut: santeInaptitudeEps.dateDebut,
      dateFin: santeInaptitudeEps.dateFin,
      motif: santeInaptitudeEps.motif,
      libelleExtrait: santeInaptitudeEps.libelleExtrait,
      documentId: santeInaptitudeEps.documentId,
      extraitId: santeInaptitudeEps.extraitId,
      actif: santeInaptitudeEps.actif,
      auteurNom: santeInaptitudeEps.auteurNom,
    });

  return mapRow({
    ...row!,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
  });
}

/** Soft : désactive l’inaptitude et l’extrait EPS lié. Pas de DELETE. */
export async function desactiverSanteInaptitudeEps(
  etablissementId: string,
  id: string,
): Promise<void> {
  const db = getDb();
  const [existing] = await db
    .select({
      id: santeInaptitudeEps.id,
      extraitId: santeInaptitudeEps.extraitId,
      actif: santeInaptitudeEps.actif,
    })
    .from(santeInaptitudeEps)
    .where(
      and(eq(santeInaptitudeEps.etablissementId, etablissementId), eq(santeInaptitudeEps.id, id)),
    )
    .limit(1);
  if (!existing) throw new Error("Inaptitude introuvable.");
  if (!existing.actif) return;

  const now = new Date();
  await db
    .update(santeInaptitudeEps)
    .set({ actif: false, updatedAt: now })
    .where(
      and(eq(santeInaptitudeEps.etablissementId, etablissementId), eq(santeInaptitudeEps.id, id)),
    );

  if (existing.extraitId) {
    const [ex] = await db
      .select({ id: santeExtrait.id, actif: santeExtrait.actif })
      .from(santeExtrait)
      .where(
        and(
          eq(santeExtrait.etablissementId, etablissementId),
          eq(santeExtrait.id, existing.extraitId),
        ),
      )
      .limit(1);
    if (ex?.actif) {
      await desactiverSanteExtrait(etablissementId, existing.extraitId);
    }
  }
}
