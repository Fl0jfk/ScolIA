import "server-only";

import { eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refNomenclature } from "@/db/schema";
import type { SiecleXmlKind } from "@/app/lib/nomenclature-import/siecle-xml";
import {
  isSiecleImportCycle,
  SIECLE_IMPORT_CYCLES,
  type SiecleImportCycle,
} from "@/app/lib/nomenclature-import/siecle-import-cycle";
import { loadOfficialSchoolClasses } from "@/app/lib/nomenclature-classes";

export type SiecleImportSlot = {
  kind: SiecleXmlKind;
  label: string;
  filenameHint: string;
  required: boolean;
  order: number;
  /** Toujours true : Siècle exporte collège et lycée séparément. */
  cycleScoped: boolean;
};

export const SIECLE_IMPORT_SLOTS: SiecleImportSlot[] = [
  {
    kind: "communs",
    label: "Communs",
    filenameHint: "Communs.xml",
    required: true,
    order: 0,
    cycleScoped: true,
  },
  {
    kind: "nomenclature",
    label: "Nomenclature",
    filenameHint: "Nomenclature.xml",
    required: true,
    order: 1,
    cycleScoped: true,
  },
  {
    kind: "geographique",
    label: "Géographique",
    filenameHint: "Geographique.xml",
    required: false,
    order: 2,
    cycleScoped: true,
  },
  {
    kind: "etablissements",
    label: "Établissements",
    filenameHint: "Etablissements.xml",
    required: false,
    order: 3,
    cycleScoped: true,
  },
  {
    kind: "structures",
    label: "Structures",
    filenameHint: "Structures.xml",
    required: true,
    order: 4,
    cycleScoped: true,
  },
  {
    kind: "eleves",
    label: "Élèves",
    filenameHint: "ElevesSansAdresses.xml",
    required: false,
    order: 5,
    cycleScoped: true,
  },
  {
    kind: "responsables",
    label: "Responsables (parents + adresses)",
    filenameHint: "ResponsablesAvecAdresses.xml",
    required: false,
    order: 6,
    cycleScoped: true,
  },
];

type RapportJson = Record<string, unknown>;

function asRapport(raw: unknown): RapportJson | null {
  if (!raw || typeof raw !== "object") return null;
  return raw as RapportJson;
}

function rapportCycle(rapport: RapportJson | null): SiecleImportCycle | null {
  const raw = rapport?.cycle;
  return isSiecleImportCycle(raw) ? raw : null;
}

export type SiecleImportStatus = {
  kind: SiecleXmlKind;
  cycle: SiecleImportCycle;
  imported: boolean;
  lastImport: string | null;
  lastFile: string | null;
  statut: string | null;
  rows: number | null;
};

function dateToIso(value: unknown): string | null {
  if (value instanceof Date) return value.toISOString();
  if (typeof value === "string") return value;
  return null;
}

const GEO_TYPES = new Set(["pays", "departement", "commune"]);
const STRUCTURE_TYPES = new Set(["division"]);

function buildSlotStatus(params: {
  slot: SiecleImportSlot;
  cycle: SiecleImportCycle;
  logs: Array<{
    fichier: string;
    statut: string;
    dateImport: unknown;
    rapportJson: unknown;
  }>;
  countByTypeCycle: Map<string, number>;
  divisionCountByCycle: Partial<Record<SiecleImportCycle, number>>;
}): SiecleImportStatus {
  const { slot, cycle, logs, countByTypeCycle, divisionCountByCycle } = params;

  const matching = [...logs].reverse().filter((l) => {
    const rapport = asRapport(l.rapportJson);
    if (rapport?.kind !== slot.kind || l.statut === "ignore") return false;
    const logCycle = rapportCycle(rapport);
    // Logs sans cycle (imports historiques) : visibles pour les deux cycles.
    return logCycle == null || logCycle === cycle;
  });

  const last = matching[0];
  const rapport = asRapport(last?.rapportJson);
  const rowsFromLog = rapport?.rows != null ? Number(rapport.rows) : null;

  let rows = rowsFromLog;
  if (slot.kind === "structures") {
    rows = divisionCountByCycle[cycle] ?? rowsFromLog;
  } else if (slot.kind === "nomenclature") {
    let total = 0;
    for (const [key, n] of countByTypeCycle) {
      const [type, rowCycle] = key.split("\0");
      if (!type || STRUCTURE_TYPES.has(type) || GEO_TYPES.has(type)) continue;
      if (rowCycle === cycle || rowCycle === "") total += n;
    }
    rows = total || rowsFromLog;
  } else if (slot.kind === "geographique") {
    let geoTotal = 0;
    for (const geoType of GEO_TYPES) {
      geoTotal += countByTypeCycle.get(`${geoType}\0${cycle}`) ?? 0;
      geoTotal += countByTypeCycle.get(`${geoType}\0`) ?? 0;
    }
    rows = geoTotal || rowsFromLog;
  }

  if (slot.kind === "eleves" && rapport?.total != null) {
    rows = Number(rapport.total);
  }

  const hasDataInDb = rows != null && rows > 0;
  const imported = Boolean((last && (last.statut === "ok" || last.statut === "partiel")) || hasDataInDb);

  return {
    kind: slot.kind,
    cycle,
    imported,
    lastImport: dateToIso(last?.dateImport),
    lastFile: last?.fichier ?? null,
    statut: last?.statut ?? null,
    rows,
  };
}

export async function buildSiecleImportStatus(etablissementId: string): Promise<SiecleImportStatus[]> {
  const db = getDb();
  const [logs, counts, official] = await Promise.all([
    db
      .select({
        fichier: nomenclatureImportLog.fichier,
        statut: nomenclatureImportLog.statut,
        dateImport: nomenclatureImportLog.dateImport,
        rapportJson: nomenclatureImportLog.rapportJson,
      })
      .from(nomenclatureImportLog)
      .where(eq(nomenclatureImportLog.etablissementId, etablissementId))
      .orderBy(nomenclatureImportLog.dateImport),
    db
      .select({
        type: refNomenclature.type,
        cycle: refNomenclature.cycle,
        n: sql<number>`count(*)::int`,
      })
      .from(refNomenclature)
      .where(eq(refNomenclature.etablissementId, etablissementId))
      .groupBy(refNomenclature.type, refNomenclature.cycle),
    loadOfficialSchoolClasses(etablissementId),
  ]);

  const countByTypeCycle = new Map(counts.map((c) => [`${c.type}\0${c.cycle ?? ""}`, c.n]));
  const divisionCountByCycle: Partial<Record<SiecleImportCycle, number>> = {
    college: official.lockedClassesByPole.COLLÈGE?.length ?? 0,
    lycee: official.lockedClassesByPole.LYCÉE?.length ?? 0,
  };

  const out: SiecleImportStatus[] = [];
  for (const slot of SIECLE_IMPORT_SLOTS) {
    for (const cycle of SIECLE_IMPORT_CYCLES) {
      out.push(
        buildSlotStatus({
          slot,
          cycle,
          logs,
          countByTypeCycle,
          divisionCountByCycle,
        }),
      );
    }
  }

  return out;
}
