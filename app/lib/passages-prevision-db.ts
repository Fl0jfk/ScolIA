import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite, passage } from "@/db/schema";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import {
  MEAL_DAY_ORDER,
  mealDayKeyFromIsoDate,
  parseEleveGrilleRepas,
} from "@/app/lib/eleve-grille-repas";
import { listEleveIdsEnSortieOnDate } from "@/app/lib/occupancy/travels-read";
import {
  droitRepasFromRegime,
  repasServiceFromHorodatage,
  resumeFromLignes,
  statutPrevision,
  type PrevisionLigne,
  type PrevisionRepasPayload,
  type PrevisionSourceDroit,
  type RepasService,
  REPAS_SERVICES,
} from "@/app/lib/passages-prevision-shared";

function jourLabel(jourCle: ReturnType<typeof mealDayKeyFromIsoDate>): string | null {
  if (!jourCle) return null;
  return MEAL_DAY_ORDER.find((d) => d.key === jourCle)?.label ?? jourCle;
}

/**
 * Prévision du jour : droit (grille → sinon régime) vs pris (passage self).
 * Élèves en sortie scolaire (`en_sortie`) : signalés, hors attendus cantine.
 * Week-end : résumé vide (grille Lun–Ven).
 * Insert/lecture uniquement — aucun DELETE.
 */
export async function getPrevisionRepasDuJour(
  etablissementId: string,
  opts?: { date?: string; service?: RepasService },
): Promise<PrevisionRepasPayload> {
  const date = (opts?.date ?? calendarDateKeyParis()).slice(0, 10);
  const jourCle = mealDayKeyFromIsoDate(date);
  const serviceFilter = opts?.service;

  if (!jourCle) {
    return {
      resume: resumeFromLignes(date, null, null, []),
      lignes: [],
    };
  }

  const db = getDb();
  const [eleves, enSortie] = await Promise.all([
    db
      .select({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        classe: eleve.classe,
        regime: eleve.regime,
        grilleRepas: eleveScolarite.grilleRepas,
        demiPension: eleveScolarite.demiPension,
        scolariteClasse: eleveScolarite.classe,
      })
      .from(eleve)
      .leftJoin(
        eleveScolarite,
        and(
          eq(eleveScolarite.eleveId, eleve.id),
          eq(eleveScolarite.etablissementId, etablissementId),
          eq(eleveScolarite.statut, "en_cours"),
        ),
      )
      .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.status, "inscrit"))),
    listEleveIdsEnSortieOnDate({ etablissementId, date }),
  ]);

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

  const prisByEleve = new Map<string, Set<RepasService>>();
  for (const row of selfRows) {
    if (!row.eleveId) continue;
    const iso =
      typeof row.horodatage === "string" ? row.horodatage : row.horodatage.toISOString();
    const service = repasServiceFromHorodatage(iso);
    const set = prisByEleve.get(row.eleveId) ?? new Set<RepasService>();
    set.add(service);
    prisByEleve.set(row.eleveId, set);
  }

  const lignes: PrevisionLigne[] = [];

  for (const e of eleves) {
    const classe = e.scolariteClasse ?? e.classe;
    const services = serviceFilter ? ([serviceFilter] as RepasService[]) : [...REPAS_SERVICES];

    if (enSortie.has(e.id)) {
      for (const service of services) {
        lignes.push({
          eleveId: e.id,
          nom: e.nom,
          prenom: e.prenom,
          classe,
          service,
          droit: false,
          pris: false,
          statut: "en_sortie",
          sourceDroit: null,
        });
      }
      continue;
    }

    const grille = parseEleveGrilleRepas(e.grilleRepas, { allowEmpty: true });
    let droitMidi = false;
    let droitSoir = false;
    let sourceDroit: PrevisionSourceDroit | null = null;

    if (grille) {
      droitMidi = Boolean(grille[jourCle].midi);
      droitSoir = Boolean(grille[jourCle].soir);
      sourceDroit = "grille";
    } else {
      const fromRegime = droitRepasFromRegime(e.regime, e.demiPension);
      droitMidi = fromRegime.midi;
      droitSoir = fromRegime.soir;
      if (droitMidi || droitSoir) sourceDroit = "regime";
    }

    const prisSet = prisByEleve.get(e.id) ?? new Set<RepasService>();

    for (const service of REPAS_SERVICES) {
      if (serviceFilter && service !== serviceFilter) continue;
      const droit = service === "midi" ? droitMidi : droitSoir;
      const pris = prisSet.has(service);
      const statut = statutPrevision(droit, pris);
      if (!statut) continue;
      lignes.push({
        eleveId: e.id,
        nom: e.nom,
        prenom: e.prenom,
        classe,
        service,
        droit,
        pris,
        statut,
        sourceDroit: droit ? sourceDroit : null,
      });
    }
  }

  lignes.sort((a, b) => {
    const byService = a.service.localeCompare(b.service);
    if (byService !== 0) return byService;
    const byNom = a.nom.localeCompare(b.nom, "fr");
    if (byNom !== 0) return byNom;
    return a.prenom.localeCompare(b.prenom, "fr");
  });

  return {
    resume: resumeFromLignes(date, jourCle, jourLabel(jourCle), lignes),
    lignes,
  };
}
