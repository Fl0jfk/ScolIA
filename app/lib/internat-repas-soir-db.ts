import "server-only";

import { and, asc, eq, isNull, lte, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveScolarite,
  internatAffectation,
  internatBatiment,
  internatChambre,
  passage,
} from "@/db/schema";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import {
  mealDayKeyFromIsoDate,
  parseEleveGrilleRepas,
} from "@/app/lib/eleve-grille-repas";
import { droitRepasFromRegime } from "@/app/lib/passages-prevision-shared";
import { repasServiceFromHorodatage } from "@/app/lib/passages-prevision-shared";
import { listEleveIdsHorsInternatLeJour } from "@/app/lib/internat-sorties-db";

export type InternatRepasSoirLigne = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  chambreLabel: string;
  batimentLabel: string;
  droitSoir: boolean;
  prisSoir: boolean;
  statut: "attendu_pris" | "attendu_manquant" | "imprevu" | "en_sortie";
  sourceDroit: "grille" | "regime" | null;
};

export type InternatRepasSoirPayload = {
  date: string;
  jourCle: string | null;
  attendus: number;
  pris: number;
  manquants: number;
  enSortie: number;
  lignes: InternatRepasSoirLigne[];
};

/**
 * Repas du soir pour les internes logés (lit daté).
 * Droit = grille.soir ou régime interne ; pris = passage self ≥ 16 h Paris.
 */
export async function getInternatRepasSoirDuJour(
  etablissementId: string,
  opts?: { date?: string },
): Promise<InternatRepasSoirPayload> {
  const date = (opts?.date ?? calendarDateKeyParis()).slice(0, 10);
  const jourCle = mealDayKeyFromIsoDate(date);
  const enSortie = await listEleveIdsHorsInternatLeJour(etablissementId, date);

  const db = getDb();
  const affectes = await db
    .select({
      eleveId: internatAffectation.eleveId,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      regime: eleve.regime,
      grilleRepas: eleveScolarite.grilleRepas,
      demiPension: eleveScolarite.demiPension,
      chambreLabel: internatChambre.label,
      batimentLabel: internatBatiment.label,
    })
    .from(internatAffectation)
    .innerJoin(
      eleve,
      and(eq(eleve.id, internatAffectation.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .leftJoin(
      eleveScolarite,
      and(
        eq(eleveScolarite.eleveId, eleve.id),
        eq(eleveScolarite.etablissementId, etablissementId),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
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
    .where(
      and(
        eq(internatAffectation.etablissementId, etablissementId),
        lte(internatAffectation.dateDebut, date),
        or(isNull(internatAffectation.dateFin), sql`${internatAffectation.dateFin} >= ${date}`),
      ),
    )
    .orderBy(asc(internatBatiment.label), asc(internatChambre.label), asc(eleve.nom));

  const dayStart = `${date}T00:00:00.000Z`;
  const dayEnd = `${date}T23:59:59.999Z`;
  const selfRows = await db
    .select({
      eleveId: passage.eleveId,
      horodatage: passage.horodatage,
    })
    .from(passage)
    .where(
      and(
        eq(passage.etablissementId, etablissementId),
        eq(passage.lieu, "self"),
        sql`${passage.horodatage} >= ${dayStart}::timestamptz`,
        sql`${passage.horodatage} <= ${dayEnd}::timestamptz`,
      ),
    );

  const prisSoir = new Set<string>();
  for (const r of selfRows) {
    if (!r.eleveId) continue;
    const iso = typeof r.horodatage === "string" ? r.horodatage : r.horodatage.toISOString();
    if (repasServiceFromHorodatage(iso) === "soir") prisSoir.add(r.eleveId);
  }

  const lignes: InternatRepasSoirLigne[] = [];
  for (const a of affectes) {
    if (enSortie.has(a.eleveId)) {
      lignes.push({
        eleveId: a.eleveId,
        nom: a.nom,
        prenom: a.prenom,
        classe: a.classe,
        chambreLabel: a.chambreLabel,
        batimentLabel: a.batimentLabel,
        droitSoir: false,
        prisSoir: false,
        statut: "en_sortie",
        sourceDroit: null,
      });
      continue;
    }

    let droitSoir = false;
    let sourceDroit: "grille" | "regime" | null = null;
    if (jourCle) {
      const grille = parseEleveGrilleRepas(a.grilleRepas, { allowEmpty: true });
      if (grille) {
        droitSoir = Boolean(grille[jourCle].soir);
        sourceDroit = "grille";
      } else {
        const d = droitRepasFromRegime(a.regime, a.demiPension);
        droitSoir = d.soir;
        if (droitSoir) sourceDroit = "regime";
      }
    }

    const pris = prisSoir.has(a.eleveId);
    let statut: InternatRepasSoirLigne["statut"];
    if (droitSoir && pris) statut = "attendu_pris";
    else if (droitSoir && !pris) statut = "attendu_manquant";
    else if (!droitSoir && pris) statut = "imprevu";
    else continue;

    lignes.push({
      eleveId: a.eleveId,
      nom: a.nom,
      prenom: a.prenom,
      classe: a.classe,
      chambreLabel: a.chambreLabel,
      batimentLabel: a.batimentLabel,
      droitSoir,
      prisSoir: pris,
      statut,
      sourceDroit: droitSoir ? sourceDroit : null,
    });
  }

  let attendus = 0;
  let pris = 0;
  let manquants = 0;
  let enSortieN = 0;
  for (const l of lignes) {
    if (l.statut === "en_sortie") enSortieN += 1;
    if (l.droitSoir) attendus += 1;
    if (l.prisSoir) pris += 1;
    if (l.statut === "attendu_manquant") manquants += 1;
  }

  return {
    date,
    jourCle,
    attendus,
    pris,
    manquants,
    enSortie: enSortieN,
    lignes,
  };
}
