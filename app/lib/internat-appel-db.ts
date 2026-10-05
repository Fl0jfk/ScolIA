import "server-only";

import { and, asc, eq, isNull, lte, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  internatAffectation,
  internatAppel,
  internatAppelLigne,
  internatBatiment,
  internatChambre,
} from "@/db/schema";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { listEleveIdsHorsInternatLeJour } from "@/app/lib/internat-sorties-db";
import {
  isInternatAppelMarque,
  type InternatAppelLigneRow,
  type InternatAppelMarque,
  type InternatAppelResume,
  type InternatAppelSoirPayload,
  type InternatAppelStatut,
} from "@/app/lib/internat-appel-shared";

function toStatut(raw: string): InternatAppelStatut {
  return raw === "validee" ? "validee" : "ouverte";
}

function toMarque(raw: string): InternatAppelMarque {
  return isInternatAppelMarque(raw) ? raw : "present";
}

function resumeFrom(
  head: {
    id: string;
    dateAppel: string | Date;
    statut: string;
    batimentId: string | null;
    valideAt: Date | string | null;
  },
  lignes: InternatAppelLigneRow[],
): InternatAppelResume {
  let presents = 0;
  let absents = 0;
  let excuses = 0;
  let activites = 0;
  for (const l of lignes) {
    if (l.marque === "present") presents += 1;
    else if (l.marque === "absent") absents += 1;
    else if (l.marque === "excuse") excuses += 1;
    else activites += 1;
  }
  return {
    id: head.id,
    dateAppel: String(head.dateAppel).slice(0, 10),
    statut: toStatut(head.statut),
    batimentId: head.batimentId,
    valideAt: head.valideAt
      ? typeof head.valideAt === "string"
        ? head.valideAt
        : head.valideAt.toISOString()
      : null,
    total: lignes.length,
    presents,
    absents,
    excuses,
    activites,
  };
}

/** Élèves avec lit ouvert couvrant la date (date_debut ≤ jour ≤ date_fin ou ouverte). */
async function rosterAffectationsDuJour(
  etablissementId: string,
  dateAppel: string,
  batimentId?: string | null,
): Promise<
  Array<{
    eleveId: string;
    eleveNom: string;
    elevePrenom: string;
    eleveClasse: string | null;
    chambreLabel: string;
    batimentLabel: string;
    batimentId: string;
  }>
> {
  const db = getDb();
  const conditions = [
    eq(internatAffectation.etablissementId, etablissementId),
    lte(internatAffectation.dateDebut, dateAppel),
    or(isNull(internatAffectation.dateFin), sql`${internatAffectation.dateFin} >= ${dateAppel}`),
  ];
  if (batimentId) {
    conditions.push(eq(internatChambre.batimentId, batimentId));
  }

  const rows = await db
    .select({
      eleveId: internatAffectation.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      chambreLabel: internatChambre.label,
      batimentLabel: internatBatiment.label,
      batimentId: internatBatiment.id,
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
    .where(and(...conditions))
    .orderBy(asc(internatBatiment.label), asc(internatChambre.label), asc(eleve.nom));

  return rows;
}

async function loadLignes(
  etablissementId: string,
  appelId: string,
): Promise<InternatAppelLigneRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: internatAppelLigne.id,
      eleveId: internatAppelLigne.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      chambreLabel: internatChambre.label,
      batimentLabel: internatBatiment.label,
      marque: internatAppelLigne.marque,
      note: internatAppelLigne.note,
    })
    .from(internatAppelLigne)
    .innerJoin(
      eleve,
      and(eq(eleve.id, internatAppelLigne.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .leftJoin(
      internatAffectation,
      and(
        eq(internatAffectation.eleveId, internatAppelLigne.eleveId),
        eq(internatAffectation.etablissementId, etablissementId),
        isNull(internatAffectation.dateFin),
      ),
    )
    .leftJoin(
      internatChambre,
      and(
        eq(internatChambre.id, internatAffectation.chambreId),
        eq(internatChambre.etablissementId, etablissementId),
      ),
    )
    .leftJoin(
      internatBatiment,
      and(
        eq(internatBatiment.id, internatChambre.batimentId),
        eq(internatBatiment.etablissementId, etablissementId),
      ),
    )
    .where(
      and(
        eq(internatAppelLigne.etablissementId, etablissementId),
        eq(internatAppelLigne.appelId, appelId),
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));

  return rows.map((r) => ({
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    chambreLabel: r.chambreLabel,
    batimentLabel: r.batimentLabel,
    marque: toMarque(r.marque),
    note: r.note,
  }));
}

/**
 * Ouvre ou récupère l’appel du soir (batimentId null = global).
 * Synchronise les lignes avec les lits datés du jour — upsert unitaire, pas de DELETE massif.
 */
export async function getOrOpenAppelSoir(
  etablissementId: string,
  opts?: { date?: string; batimentId?: string | null },
): Promise<InternatAppelSoirPayload> {
  const db = getDb();
  const dateAppel = (opts?.date ?? calendarDateKeyParis()).slice(0, 10);
  const batimentId = opts?.batimentId?.trim() || null;

  let head =
    (
      await db
        .select()
        .from(internatAppel)
        .where(
          and(
            eq(internatAppel.etablissementId, etablissementId),
            eq(internatAppel.dateAppel, dateAppel),
            batimentId
              ? eq(internatAppel.batimentId, batimentId)
              : isNull(internatAppel.batimentId),
          ),
        )
        .limit(1)
    )[0] ?? null;

  if (!head) {
    const [created] = await db
      .insert(internatAppel)
      .values({
        etablissementId,
        dateAppel,
        batimentId,
        statut: "ouverte",
      })
      .returning();
    if (!created) throw new Error("Ouverture de l’appel impossible.");
    head = created;
  }

  const rosterAll = await rosterAffectationsDuJour(etablissementId, dateAppel, batimentId);
  const horsInternat = await listEleveIdsHorsInternatLeJour(etablissementId, dateAppel);
  const roster = rosterAll.filter((r) => !horsInternat.has(r.eleveId));

  if (head.statut === "ouverte") {
    for (const r of roster) {
      await db
        .insert(internatAppelLigne)
        .values({
          etablissementId,
          appelId: head.id,
          eleveId: r.eleveId,
          marque: "present",
        })
        .onConflictDoNothing({
          target: [
            internatAppelLigne.etablissementId,
            internatAppelLigne.appelId,
            internatAppelLigne.eleveId,
          ],
        });
    }
  }

  const lignesAll = await loadLignes(etablissementId, head.id);
  /** Masque les lignes d’élèves partis (week-end ou voyage) — pas de DELETE. */
  const lignes = lignesAll.filter((l) => !horsInternat.has(l.eleveId));
  return { appel: resumeFrom(head, lignes), lignes };
}

export async function setAppelSoirMarque(
  etablissementId: string,
  opts: {
    appelId: string;
    eleveId: string;
    marque: string;
    note?: string | null;
  },
): Promise<InternatAppelLigneRow> {
  if (!isInternatAppelMarque(opts.marque)) {
    throw new Error("Marque invalide (present | absent | excuse | activite).");
  }
  const db = getDb();
  const [appel] = await db
    .select({ id: internatAppel.id, statut: internatAppel.statut })
    .from(internatAppel)
    .where(
      and(
        eq(internatAppel.etablissementId, etablissementId),
        eq(internatAppel.id, opts.appelId),
      ),
    )
    .limit(1);
  if (!appel) throw new Error("Appel introuvable.");
  if (appel.statut !== "ouverte") throw new Error("Appel déjà validé — marquage impossible.");

  const [row] = await db
    .insert(internatAppelLigne)
    .values({
      etablissementId,
      appelId: opts.appelId,
      eleveId: opts.eleveId,
      marque: opts.marque,
      note: opts.note?.trim() || null,
      updatedAt: new Date(),
    })
    .onConflictDoUpdate({
      target: [
        internatAppelLigne.etablissementId,
        internatAppelLigne.appelId,
        internatAppelLigne.eleveId,
      ],
      set: {
        marque: opts.marque,
        note: opts.note?.trim() || null,
        updatedAt: new Date(),
      },
    })
    .returning({
      id: internatAppelLigne.id,
      eleveId: internatAppelLigne.eleveId,
      marque: internatAppelLigne.marque,
      note: internatAppelLigne.note,
    });

  if (!row) throw new Error("Marquage impossible.");

  const [e] = await db
    .select({ nom: eleve.nom, prenom: eleve.prenom, classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, row.eleveId)))
    .limit(1);

  return {
    id: row.id,
    eleveId: row.eleveId,
    eleveNom: e?.nom ?? "",
    elevePrenom: e?.prenom ?? "",
    eleveClasse: e?.classe ?? null,
    chambreLabel: null,
    batimentLabel: null,
    marque: toMarque(row.marque),
    note: row.note,
  };
}

/** Valide l’appel du soir. Update unitaire — pas de DELETE. */
export async function validerAppelSoir(
  etablissementId: string,
  appelId: string,
): Promise<InternatAppelSoirPayload> {
  const db = getDb();
  const [updated] = await db
    .update(internatAppel)
    .set({
      statut: "validee",
      valideAt: new Date(),
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(internatAppel.etablissementId, etablissementId),
        eq(internatAppel.id, appelId),
        eq(internatAppel.statut, "ouverte"),
      ),
    )
    .returning();
  if (!updated) throw new Error("Appel introuvable ou déjà validé.");

  const lignes = await loadLignes(etablissementId, updated.id);
  return { appel: resumeFrom(updated, lignes), lignes };
}
