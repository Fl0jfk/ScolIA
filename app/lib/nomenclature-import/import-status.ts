import "server-only";

import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refNomenclature } from "@/db/schema";
import { sql } from "drizzle-orm";
import type { SiecleXmlKind } from "@/app/lib/nomenclature-import/siecle-xml";

export type SiecleImportSlot = {
  kind: SiecleXmlKind;
  label: string;
  filenameHint: string;
  required: boolean;
  order: number;
};

export const SIECLE_IMPORT_SLOTS: SiecleImportSlot[] = [
  { kind: "communs", label: "Communs", filenameHint: "Communs.xml", required: true, order: 0 },
  {
    kind: "nomenclature",
    label: "Nomenclature",
    filenameHint: "Nomenclature.xml",
    required: true,
    order: 1,
  },
  {
    kind: "geographique",
    label: "Géographique",
    filenameHint: "Geographique.xml",
    required: false,
    order: 2,
  },
  {
    kind: "etablissements",
    label: "Établissements",
    filenameHint: "Etablissements.xml",
    required: false,
    order: 3,
  },
  {
    kind: "structures",
    label: "Structures",
    filenameHint: "Structures.xml",
    required: true,
    order: 4,
  },
  {
    kind: "eleves",
    label: "Élèves",
    filenameHint: "ElevesSansAdresses.xml",
    required: false,
    order: 5,
  },
  {
    kind: "responsables",
    label: "Responsables",
    filenameHint: "ResponsablesAvecAdresses.xml",
    required: false,
    order: 6,
  },
];

type RapportJson = Record<string, unknown>;

function asRapport(raw: unknown): RapportJson | null {
  if (!raw || typeof raw !== "object") return null;
  return raw as RapportJson;
}

export type SiecleImportStatus = {
  kind: SiecleXmlKind;
  imported: boolean;
  lastImport: string | null;
  lastFile: string | null;
  statut: string | null;
  rows: number | null;
};

export async function buildSiecleImportStatus(etablissementId: string): Promise<SiecleImportStatus[]> {
  const db = getDb();
  const logs = await db
    .select({
      fichier: nomenclatureImportLog.fichier,
      statut: nomenclatureImportLog.statut,
      dateImport: nomenclatureImportLog.dateImport,
      rapportJson: nomenclatureImportLog.rapportJson,
    })
    .from(nomenclatureImportLog)
    .where(eq(nomenclatureImportLog.etablissementId, etablissementId))
    .orderBy(nomenclatureImportLog.dateImport);

  const counts = await db
    .select({
      type: refNomenclature.type,
      n: sql<number>`count(*)::int`,
    })
    .from(refNomenclature)
    .where(eq(refNomenclature.etablissementId, etablissementId))
    .groupBy(refNomenclature.type);

  const countByType = new Map(counts.map((c) => [c.type, c.n]));

  return SIECLE_IMPORT_SLOTS.map((slot) => {
    const matching = [...logs]
      .reverse()
      .filter((l) => asRapport(l.rapportJson)?.kind === slot.kind && l.statut !== "ignore");

    const last = matching[0];
    const rapport = asRapport(last?.rapportJson);
    const rowsFromLog = rapport?.rows != null ? Number(rapport.rows) : null;

    let rows = rowsFromLog;
    if (slot.kind === "nomenclature" || slot.kind === "geographique" || slot.kind === "structures") {
      if (slot.kind === "structures") {
        rows = countByType.get("division") ?? rowsFromLog;
      } else if (slot.kind === "nomenclature") {
        const total = [...countByType.entries()]
          .filter(([t]) => t !== "division" && t !== "commune" && t !== "pays" && t !== "departement")
          .reduce((acc, [, n]) => acc + n, 0);
        rows = total || rowsFromLog;
      } else if (slot.kind === "geographique") {
        const geoTotal =
          (countByType.get("pays") ?? 0) +
          (countByType.get("departement") ?? 0) +
          (countByType.get("commune") ?? 0);
        rows = geoTotal || rowsFromLog;
      }
    }

    if (slot.kind === "eleves" && rapport?.total != null) {
      rows = Number(rapport.total);
    }

    const hasDataInDb = rows != null && rows > 0;
    const imported = Boolean((last && last.statut === "ok") || hasDataInDb);

    return {
      kind: slot.kind,
      imported,
      lastImport:
        last?.dateImport instanceof Date
          ? last.dateImport.toISOString()
          : typeof last?.dateImport === "string"
            ? last.dateImport
            : null,
      lastFile: last?.fichier ?? null,
      statut: last?.statut ?? null,
      rows,
    };
  });
}
