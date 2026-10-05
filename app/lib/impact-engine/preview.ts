import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";
import { listGroupeMembres } from "@/app/lib/groupes-pedagogiques-db";
import {
  jourSemaineFromIsoDate,
  listEdtCreneauxForJour,
} from "@/app/lib/vs-calendrier-db";
import { getTravelFromDb } from "@/app/lib/travel-db";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { assembleImpacts } from "./drawers";
import { countEnSortieOnCreneau, eachIsoDateInclusive, signalIdFor } from "./empty-slots";
import {
  clearCreneauVideSignalsForTravel,
  replaceCreneauVideSignalsForTravel,
} from "./signals";
import type {
  CreneauPopulationInput,
  CreneauVideSignal,
  VoyageImpactPreview,
} from "./types";

async function listEleveIdsForClasse(
  etablissementId: string,
  classe: string,
): Promise<string[]> {
  const db = getDb();
  const c = classe.trim();
  if (!c) return [];

  const fromScol = await db
    .select({ id: eleve.id })
    .from(eleve)
    .innerJoin(
      eleveScolarite,
      and(
        eq(eleveScolarite.eleveId, eleve.id),
        eq(eleveScolarite.etablissementId, eleve.etablissementId),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleveScolarite.classe, c)));

  const fromPlat = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        eq(eleve.classe, c),
        sql`NOT EXISTS (
          SELECT 1 FROM eleve_scolarite s
          WHERE s.etablissement_id = ${etablissementId}
            AND s.eleve_id = ${eleve.id}
            AND s.statut = 'en_cours'
        )`,
      ),
    );

  return [...new Set([...fromScol, ...fromPlat].map((r) => r.id))];
}

async function resolveExpectedEleveIds(
  etablissementId: string,
  creneau: { classe: string | null; groupeId: string | null },
): Promise<string[]> {
  if (creneau.groupeId) {
    const membres = await listGroupeMembres(etablissementId, creneau.groupeId);
    if (membres.length > 0) return membres.map((m) => m.eleveId);
  }
  if (creneau.classe?.trim()) {
    return listEleveIdsForClasse(etablissementId, creneau.classe);
  }
  return [];
}

function tripDateRange(trip: TravelsTrip): { start: string; end: string } {
  const d = trip.data || {};
  const start = String(d.startDate || d.date || "").trim();
  const end = String(d.endDate || d.startDate || d.date || start).trim();
  return { start, end };
}

function participantStats(trip: TravelsTrip): {
  participantCount: number;
  participantLinkedCount: number;
  panierRepasCount: number;
  linkedIds: string[];
} {
  const list = Array.isArray(trip.data?.participantEleves) ? trip.data.participantEleves : [];
  const linkedIds: string[] = [];
  let panierRepasCount = 0;
  for (const p of list) {
    if (p?.eleveId && String(p.eleveId).trim()) linkedIds.push(String(p.eleveId).trim());
    if (p?.panierRepas === true) panierRepasCount += 1;
  }
  return {
    participantCount: list.length,
    participantLinkedCount: linkedIds.length,
    panierRepasCount,
    linkedIds: [...new Set(linkedIds)],
  };
}

/**
 * Classes / groupes à inspecter : snapshot participants + classes live si connues.
 */
async function collectRelevantClasses(
  etablissementId: string,
  trip: TravelsTrip,
  linkedIds: string[],
): Promise<Set<string>> {
  const classes = new Set<string>();
  for (const p of trip.data?.participantEleves || []) {
    if (p?.classe?.trim()) classes.add(p.classe.trim());
  }
  if (linkedIds.length === 0) return classes;

  const db = getDb();
  const rows = await db
    .select({
      classePlat: eleve.classe,
      classeScol: eleveScolarite.classe,
    })
    .from(eleve)
    .leftJoin(
      eleveScolarite,
      and(
        eq(eleveScolarite.eleveId, eleve.id),
        eq(eleveScolarite.etablissementId, eleve.etablissementId),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
    .where(and(eq(eleve.etablissementId, etablissementId), inArray(eleve.id, linkedIds)));

  for (const r of rows) {
    const c = (r.classeScol || r.classePlat || "").trim();
    if (c) classes.add(c);
  }
  return classes;
}

async function loadCreneauxForDate(
  etablissementId: string,
  date: string,
  classes: Set<string>,
): Promise<CreneauPopulationInput[]> {
  const jour = jourSemaineFromIsoDate(date);
  const byId = new Map<string, CreneauPopulationInput>();

  const addRows = async (
    rows: Awaited<ReturnType<typeof listEdtCreneauxForJour>>,
  ) => {
    for (const row of rows) {
      if (byId.has(row.id)) continue;
      const expectedEleveIds = await resolveExpectedEleveIds(etablissementId, {
        classe: row.classe,
        groupeId: row.groupeId,
      });
      byId.set(row.id, {
        creneauId: row.id,
        jourSemaine: row.jourSemaine,
        heureDebut: row.heureDebut,
        heureFin: row.heureFin,
        classe: row.classe,
        groupeId: row.groupeId,
        enseignantNom: row.enseignantNom,
        matiereLabel: row.matiereLibelle ?? null,
        expectedEleveIds,
      });
    }
  };

  if (classes.size === 0) {
    await addRows(await listEdtCreneauxForJour(etablissementId, jour));
  } else {
    for (const classe of classes) {
      await addRows(await listEdtCreneauxForJour(etablissementId, jour, { classe }));
    }
  }

  return [...byId.values()];
}

function detectVides(opts: {
  etablissementId: string;
  travelId: string;
  date: string;
  creneaux: CreneauPopulationInput[];
  enSortieIds: ReadonlySet<string>;
  nowIso: string;
}): CreneauVideSignal[] {
  const out: CreneauVideSignal[] = [];
  for (const c of opts.creneaux) {
    const stats = countEnSortieOnCreneau({
      expectedEleveIds: c.expectedEleveIds,
      enSortieEleveIds: opts.enSortieIds,
    });
    if (!stats.vide) continue;
    out.push({
      id: signalIdFor({
        etablissementId: opts.etablissementId,
        travelId: opts.travelId,
        date: opts.date,
        creneauId: c.creneauId,
      }),
      etablissementId: opts.etablissementId,
      travelId: opts.travelId,
      date: opts.date,
      creneauId: c.creneauId,
      jourSemaine: c.jourSemaine,
      heureDebut: c.heureDebut,
      heureFin: c.heureFin,
      classe: c.classe,
      groupeId: c.groupeId,
      enseignantNom: c.enseignantNom,
      matiereLabel: c.matiereLabel ?? null,
      expectedCount: stats.expectedCount,
      enSortieCount: stats.enSortieCount,
      kind: "creneau_vide",
      createdAt: opts.nowIso,
    });
  }
  return out;
}

export type PreviewVoyageImpactsOpts = {
  etablissementId: string;
  travelId?: string;
  /** Snapshot trip (confirm hook) — sinon charge depuis la DB. */
  trip?: TravelsTrip;
  /**
   * `as_if_active` : traite les participants liés comme `en_sortie` sur les dates du voyage
   * (preview avant VALIDE ou juste après confirm).
   * `live` : union occupancy réelle (autres sorties) + participants de ce voyage.
   */
  mode?: "as_if_active" | "live";
  /** Persiste les signaux process (confirm) ou les efface (annulation). */
  persistSignals?: boolean;
};

/**
 * Preview A/B/C/D + créneaux vidés. **Zéro** écriture `teacher_planning_replacement`.
 */
export async function previewVoyageImpacts(
  opts: PreviewVoyageImpactsOpts,
): Promise<VoyageImpactPreview> {
  const etablissementId = opts.etablissementId.trim();
  if (!etablissementId) {
    throw new Error("etablissementId requis");
  }

  let trip = opts.trip;
  const travelId = (opts.travelId || trip?.id || "").trim();
  if (!trip) {
    if (!travelId) throw new Error("travelId ou trip requis");
    trip = (await getTravelFromDb(etablissementId, travelId)) ?? undefined;
    if (!trip) throw new Error("Voyage introuvable");
  }

  const id = trip.id || travelId;
  const status = String(trip.status || "");
  const cancelled = status === "ANNULE" || status === "SEANCE_ANNULEE";
  const dates = tripDateRange(trip);
  const stats = participantStats(trip);
  const nowIso = new Date().toISOString();
  const mode = opts.mode ?? "as_if_active";

  if (cancelled) {
    if (opts.persistSignals) {
      clearCreneauVideSignalsForTravel(etablissementId, id);
    }
    const impacts = assembleImpacts({
      participantCount: stats.participantCount,
      participantLinkedCount: stats.participantLinkedCount,
      panierRepasCount: stats.panierRepasCount,
      creneauxVidesCount: 0,
      status,
      edtCoverage: "partial",
    });
    return {
      etablissementId,
      travelId: id,
      title: trip.data?.title ? String(trip.data.title) : null,
      dates,
      status,
      participantCount: stats.participantCount,
      participantLinkedCount: stats.participantLinkedCount,
      panierRepasCount: stats.panierRepasCount,
      impacts,
      creneauxVides: [],
      wroteTeacherPlanningReplacement: false,
      coverage: "partial",
    };
  }

  const dateList = eachIsoDateInclusive(dates.start, dates.end);
  const classes = await collectRelevantClasses(etablissementId, trip, stats.linkedIds);
  const enSortieBase = new Set(stats.linkedIds);

  if (mode === "live" && dateList.length > 0 && classes.size > 0) {
    try {
      const { occupancy } = await import("@/app/lib/occupancy/port");
      for (const date of dateList) {
        for (const classe of classes) {
          const r = await occupancy({ etablissementId, date, classe });
          for (const f of r.facts) {
            if (f.tag === "en_sortie") enSortieBase.add(f.eleveId);
          }
        }
      }
    } catch {
      /* occupancy indisponible → on garde les participants du voyage */
    }
  }

  const creneauxVides: CreneauVideSignal[] = [];
  let sawAnyCreneau = false;

  for (const date of dateList) {
    const creneaux = await loadCreneauxForDate(etablissementId, date, classes);
    if (creneaux.length > 0) sawAnyCreneau = true;

    // Pour as_if_active : participants liés = en_sortie ce jour-là.
    // Pour live : déjà enrichi ci-dessus ; on unionne toujours les participants du voyage.
    const enSortieIds = new Set(enSortieBase);

    creneauxVides.push(
      ...detectVides({
        etablissementId,
        travelId: id,
        date,
        creneaux,
        enSortieIds,
        nowIso,
      }),
    );
  }

  if (opts.persistSignals) {
    replaceCreneauVideSignalsForTravel(etablissementId, id, creneauxVides);
  }

  const edtCoverage: "complete" | "partial" | "unavailable" =
    dateList.length === 0
      ? "unavailable"
      : sawAnyCreneau
        ? classes.size > 0
          ? "complete"
          : "partial"
        : "unavailable";

  const impacts = assembleImpacts({
    participantCount: stats.participantCount,
    participantLinkedCount: stats.participantLinkedCount,
    panierRepasCount: stats.panierRepasCount,
    creneauxVidesCount: creneauxVides.length,
    status,
    edtCoverage,
  });

  return {
    etablissementId,
    travelId: id,
    title: trip.data?.title ? String(trip.data.title) : null,
    dates,
    status,
    participantCount: stats.participantCount,
    participantLinkedCount: stats.participantLinkedCount,
    panierRepasCount: stats.panierRepasCount,
    impacts,
    creneauxVides,
    wroteTeacherPlanningReplacement: false,
    coverage:
      stats.participantLinkedCount < stats.participantCount
        ? "partial"
        : edtCoverage === "unavailable"
          ? "partial"
          : "complete",
  };
}
