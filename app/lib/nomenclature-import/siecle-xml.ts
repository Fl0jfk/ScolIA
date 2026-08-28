/**
 * Détection + import XML Siècle (ISO-8859-15).
 * Pont EN complet : Communs, Nomenclature, Géographique, Établissements, Structures, Élèves, Responsables.
 */
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refNomenclature } from "@/db/schema";
import {
  extractSiecleEleveIdMapFromXml,
  importSiecleElevesXml,
  importSiecleResponsablesXml,
} from "@/app/lib/nomenclature-import/siecle-people-import";
import { upsertGroupesFromSiecleCodes } from "@/app/lib/groupes-pedagogiques-db";
import { parseGeographiqueXml } from "@/app/lib/nomenclature-import/siecle-geographique-parse";
import { importSiecleEtablissementsXml } from "@/app/lib/nomenclature-import/siecle-etablissements-import";
import { importSiecleCommunsXml } from "@/app/lib/nomenclature-import/siecle-communs-import";
import { syncProgrammesToCompetences } from "@/app/lib/nomenclature-import/sync-programmes-competences";
import { syncMatieresFromNomenclature } from "@/app/lib/nomenclature-import/sync-matieres-from-nomenclature";
import { normalizeElevesToSiecleClasses } from "@/app/lib/nomenclature-import/normalize-eleves-classes";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import { isEntCoreDbEnabled } from "@/app/lib/ent-core-db";
import {
  parseNomenclatureXml,
  parseStructuresDivisions,
  parseStructuresGroupes,
} from "@/app/lib/nomenclature-import/siecle-nomenclature-parse";
import { decodeSiecleBuffer } from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";

export { decodeSiecleBuffer } from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";

export type SiecleXmlKind =
  | "nomenclature"
  | "geographique"
  | "etablissements"
  | "structures"
  | "eleves"
  | "responsables"
  | "communs"
  | "inconnu";

export function detectSiecleXmlKind(xml: string): SiecleXmlKind {
  const head = xml.slice(0, 4000).toUpperCase();
  if (head.includes("BEE_NOMENCLATURES") || head.includes("<NOMENCLATURE")) return "nomenclature";
  if (head.includes("BEE_GEOGRAPHIQUE")) return "geographique";
  if (head.includes("BEE_ETABLISSEMENTS")) return "etablissements";
  if (head.includes("BEE_STRUCTURES")) return "structures";
  if (head.includes("BEE_ELEVES") || head.includes("ELEVESSANSADRESSES")) return "eleves";
  if (head.includes("BEE_RESPONSABLES") || head.includes("RESPONSABLESAVECADRESSES")) {
    return "responsables";
  }
  if (head.includes("BEE_COMMUN") || head.includes("<UAJ")) return "communs";
  return "inconnu";
}

import type { NomenclatureUpsertRow } from "@/app/lib/nomenclature-import/siecle-xml-types";

export type { NomenclatureUpsertRow } from "@/app/lib/nomenclature-import/siecle-xml-types";
export { parseNomenclatureXml, parseStructuresDivisions, parseStructuresGroupes } from "@/app/lib/nomenclature-import/siecle-nomenclature-parse";

export async function upsertNomenclatureRows(
  etablissementId: string,
  rows: NomenclatureUpsertRow[],
  source = "siecle",
): Promise<{ inserts: number; updates: number }> {
  const db = getDb();
  let inserts = 0;
  let updates = 0;
  const chunk = 200;
  for (let i = 0; i < rows.length; i += chunk) {
    const slice = rows.slice(i, i + chunk);
    for (const r of slice) {
      const existing = await db
        .select({ id: refNomenclature.id })
        .from(refNomenclature)
        .where(
          and(
            eq(refNomenclature.etablissementId, etablissementId),
            eq(refNomenclature.type, r.type),
            eq(refNomenclature.code, r.code),
          ),
        )
        .limit(1);

      if (existing[0]) {
        await db
          .update(refNomenclature)
          .set({
            libelleCourt: r.libelleCourt || null,
            libelleLong: r.libelleLong || null,
            metadataJson: r.metadataJson || null,
            validFrom: r.validFrom || null,
            validTo: r.validTo || null,
            source,
            updatedAt: new Date(),
          })
          .where(eq(refNomenclature.id, existing[0].id));
        updates += 1;
      } else {
        await db.insert(refNomenclature).values({
          etablissementId,
          type: r.type,
          code: r.code,
          libelleCourt: r.libelleCourt || null,
          libelleLong: r.libelleLong || null,
          metadataJson: r.metadataJson || null,
          validFrom: r.validFrom || null,
          validTo: r.validTo || null,
          source,
        });
        inserts += 1;
      }
    }
  }
  return { inserts, updates };
}

export function rankSiecleImportFilename(name: string): number {
  const n = name.toLowerCase();
  if (n.includes("commun")) return 0;
  if (n.includes("nomenclature")) return 1;
  if (n.includes("geograph")) return 2;
  if (n.includes("structure")) return 3;
  if (n.includes("eleve")) return 4;
  if (n.includes("responsable")) return 5;
  if (n.includes("etablissement")) return 6;
  return 4;
}

export async function importSiecleXmlBuffersBatch(
  etablissementId: string,
  files: Array<{ filename: string; buffer: ArrayBuffer | Buffer | Uint8Array }>,
): Promise<
  Array<{
    file: string;
    kind?: string;
    message?: string;
    error?: string;
    inserts?: number;
    updates?: number;
  }>
> {
  const sorted = [...files].sort(
    (a, b) => rankSiecleImportFilename(a.filename) - rankSiecleImportFilename(b.filename),
  );
  const reports: Array<{
    file: string;
    kind?: string;
    message?: string;
    error?: string;
    inserts?: number;
    updates?: number;
  }> = [];
  let batchEleveIdMap: Record<string, string> = {};

  for (const file of sorted) {
    if (!/\.xml$/i.test(file.filename)) {
      reports.push({ file: file.filename, error: "Extension .xml attendue." });
      continue;
    }
    try {
      const report = await importSiecleXmlBuffer(etablissementId, file.filename, file.buffer, {
        inlineEleveIdMap: batchEleveIdMap,
      });
      if (report.eleveIdMap) {
        batchEleveIdMap = { ...batchEleveIdMap, ...report.eleveIdMap };
      }
      reports.push({
        file: file.filename,
        kind: report.kind,
        message: report.message,
        inserts: report.inserts,
        updates: report.updates,
      });
    } catch (e) {
      reports.push({
        file: file.filename,
        error: e instanceof Error ? e.message : "Import impossible.",
      });
    }
  }

  return reports;
}

export async function importSiecleXmlBuffer(
  etablissementId: string,
  filename: string,
  buffer: ArrayBuffer | Buffer | Uint8Array,
  opts?: { inlineEleveIdMap?: Record<string, string> },
): Promise<{
  kind: SiecleXmlKind;
  inserts: number;
  updates: number;
  rows: number;
  message: string;
  eleveIdMap?: Record<string, string>;
}> {
  const xml = decodeSiecleBuffer(buffer);
  const kind = detectSiecleXmlKind(xml);

  if (kind === "eleves") {
    const result = await importSiecleElevesXml(etablissementId, filename, xml);
    return {
      kind,
      inserts: result.inserts,
      updates: result.updates,
      rows: result.rows,
      message: result.message,
      eleveIdMap: extractSiecleEleveIdMapFromXml(xml),
    };
  }

  if (kind === "responsables") {
    const result = await importSiecleResponsablesXml(
      etablissementId,
      filename,
      xml,
      opts?.inlineEleveIdMap,
    );
    return {
      kind,
      inserts: result.inserts,
      updates: result.updates,
      rows: result.rows,
      message: result.message,
    };
  }

  if (kind === "communs") {
    const result = await importSiecleCommunsXml(etablissementId, filename, xml);
    return {
      kind,
      inserts: result.inserts,
      updates: result.updates,
      rows: result.rows,
      message: result.message,
    };
  }

  if (kind === "etablissements") {
    const result = await importSiecleEtablissementsXml(etablissementId, filename, xml);
    return {
      kind,
      inserts: result.inserts,
      updates: result.updates,
      rows: result.rows,
      message: result.message,
    };
  }

  let rows: NomenclatureUpsertRow[] = [];

  if (kind === "nomenclature") {
    rows = parseNomenclatureXml(xml);
  } else if (kind === "geographique") {
    rows = parseGeographiqueXml(xml);
  } else if (kind === "structures") {
    rows = parseStructuresDivisions(xml);
    const groupes = parseStructuresGroupes(xml);
    if (groupes.length) {
      const gResult = await upsertGroupesFromSiecleCodes(etablissementId, groupes);
      const db = getDb();
      await db.insert(nomenclatureImportLog).values({
        etablissementId,
        fichier: filename,
        statut: "ok",
        nbInserts: gResult.inserts,
        nbUpdates: gResult.updates,
        rapportJson: { kind: "structures_groupes", rows: groupes.length },
      });
    }
  } else {
    const db = getDb();
    await db.insert(nomenclatureImportLog).values({
      etablissementId,
      fichier: filename,
      statut: "ignore",
      rapportJson: {
        kind,
        note: "Type non reconnu — import pris en charge : communs, nomenclature, geographique, etablissements, structures, eleves, responsables.",
      },
    });
    return {
      kind,
      inserts: 0,
      updates: 0,
      rows: 0,
      message: `Fichier « ${kind} » détecté — type non pris en charge.`,
    };
  }

  const { inserts, updates } = await upsertNomenclatureRows(etablissementId, rows);
  const db = getDb();

  let elevesClassesNormalized: { normalized: number; unresolved: string[] } | null = null;
  if (kind === "structures") {
    try {
      const registry = await loadElevesRegistry();
      if (registry.length) {
        const norm = await normalizeElevesToSiecleClasses(etablissementId, registry);
        if (norm.normalized > 0 || norm.unresolved.length) {
          await saveElevesRegistry(norm.eleves);
        }
        elevesClassesNormalized = {
          normalized: norm.normalized,
          unresolved: norm.unresolved,
        };
      }
    } catch {
      elevesClassesNormalized = null;
    }
  }

  let competencesSynced: { domaines: number; items: number } | null = null;
  let matieresSynced: { inserts: number; updates: number } | null = null;
  if (kind === "nomenclature" && isEntCoreDbEnabled()) {
    try {
      competencesSynced = await syncProgrammesToCompetences(etablissementId);
    } catch {
      competencesSynced = null;
    }
    try {
      matieresSynced = await syncMatieresFromNomenclature(etablissementId);
    } catch {
      matieresSynced = null;
    }
  }

  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: filename,
    statut: "ok",
    nbInserts: inserts,
    nbUpdates: updates,
    rapportJson: {
      kind,
      rows: rows.length,
      ...(competencesSynced ? { competencesSynced } : {}),
      ...(matieresSynced ? { matieresSynced } : {}),
      ...(elevesClassesNormalized ? { elevesClassesNormalized } : {}),
    },
  });

  const compNote =
    competencesSynced && competencesSynced.items > 0
      ? ` · ${competencesSynced.items} programme(s) → compétences LSU`
      : "";
  const matNote =
    matieresSynced && matieresSynced.inserts + matieresSynced.updates > 0
      ? ` · ${matieresSynced.inserts + matieresSynced.updates} matière(s) → notes`
      : "";
  const classNote =
    elevesClassesNormalized && elevesClassesNormalized.normalized > 0
      ? ` · ${elevesClassesNormalized.normalized} classe(s) élève normalisée(s)`
      : "";

  return {
    kind,
    inserts,
    updates,
    rows: rows.length,
    message: `${filename} (${kind}) : ${rows.length} entrées — ${inserts} créées, ${updates} mises à jour.${compNote}${matNote}${classNote}`,
  };
}
