import "server-only";

import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refEtablissement } from "@/db/schema";
import {
  siecleCycleLabel,
  type SiecleImportCycle,
} from "@/app/lib/nomenclature-import/siecle-import-cycle";
import {
  attrValue,
  codeFromElement,
  extractSiecleElements,
  firstNonEmpty,
  parseSiecleDate,
  tagValue,
  type SiecleElement,
} from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";

export type RefEtablissementRow = {
  codeRne: string;
  codeNature?: string;
  codeType?: string;
  codeSecteur?: string;
  sigle?: string;
  denomPrinc?: string;
  denomCompl?: string;
  adresse?: string;
  dateOuverture?: string | null;
  dateFermeture?: string | null;
};

/**
 * Parse un bloc ETABLISSEMENT Siècle.
 * CODE_RNE est souvent en **attribut** (`<ETABLISSEMENT CODE_RNE="…">`), parfois en balise enfant.
 */
export function parseEtablissementElement(el: SiecleElement): RefEtablissementRow | null {
  const codeRne = firstNonEmpty(
    codeFromElement(el, ["CODE_RNE", "CODE_ETABLISSEMENT", "RNE", "UAI", "UAJ"], [
      "CODE_RNE",
      "CODE_ETABLISSEMENT",
      "RNE",
      "UAI",
      "UAJ",
    ]),
  );
  if (!codeRne) return null;

  const adresseParts = [
    tagValue(el.inner, "ADRESSE1"),
    tagValue(el.inner, "ADRESSE2"),
    tagValue(el.inner, "ADRESSE_1"),
    tagValue(el.inner, "ADRESSE_2"),
    tagValue(el.inner, "CODE_POSTAL"),
    tagValue(el.inner, "COMMUNE"),
    tagValue(el.inner, "VILLE"),
    tagValue(el.inner, "LIBELLE_COMMUNE"),
  ].filter(Boolean);

  return {
    codeRne,
    codeNature:
      firstNonEmpty(
        attrValue(el.attrs, "CODE_NATURE"),
        tagValue(el.inner, "CODE_NATURE"),
        tagValue(el.inner, "NATURE"),
      ) || undefined,
    codeType:
      firstNonEmpty(
        attrValue(el.attrs, "CODE_TYPE"),
        tagValue(el.inner, "CODE_TYPE"),
        tagValue(el.inner, "TYPE"),
      ) || undefined,
    codeSecteur:
      firstNonEmpty(
        attrValue(el.attrs, "CODE_SECTEUR"),
        tagValue(el.inner, "CODE_SECTEUR"),
        tagValue(el.inner, "SECTEUR"),
      ) || undefined,
    sigle: firstNonEmpty(attrValue(el.attrs, "SIGLE"), tagValue(el.inner, "SIGLE")) || undefined,
    denomPrinc:
      firstNonEmpty(
        tagValue(el.inner, "DENOM_PRINC"),
        tagValue(el.inner, "DENOMINATION_PRINCIPALE"),
        tagValue(el.inner, "NOM_ETABLISSEMENT"),
        tagValue(el.inner, "NOM"),
        attrValue(el.attrs, "DENOM_PRINC"),
      ) || undefined,
    denomCompl:
      firstNonEmpty(
        tagValue(el.inner, "DENOM_COMPL"),
        tagValue(el.inner, "DENOMINATION_COMPLEMENTAIRE"),
      ) || undefined,
    adresse: adresseParts.length ? adresseParts.join(", ") : undefined,
    dateOuverture: parseSiecleDate(tagValue(el.inner, "DATE_OUVERTURE")) ?? null,
    dateFermeture: parseSiecleDate(tagValue(el.inner, "DATE_FERMETURE")) ?? null,
  };
}

/** @deprecated Préférer parseEtablissementElement (attrs + inner). Conservé pour Communs. */
export function parseEtablissementBlock(block: string): RefEtablissementRow | null {
  return parseEtablissementElement({ attrs: "", inner: block });
}

export async function upsertRefEtablissementRows(
  rows: RefEtablissementRow[],
  source = "siecle",
): Promise<{ inserts: number; updates: number }> {
  const db = getDb();
  let inserts = 0;
  let updates = 0;

  for (const r of rows) {
    const [existing] = await db
      .select({ id: refEtablissement.id })
      .from(refEtablissement)
      .where(eq(refEtablissement.codeRne, r.codeRne))
      .limit(1);

    const payload = {
      codeNature: r.codeNature || null,
      codeType: r.codeType || null,
      codeSecteur: r.codeSecteur || null,
      sigle: r.sigle || null,
      denomPrinc: r.denomPrinc || null,
      denomCompl: r.denomCompl || null,
      adresse: r.adresse || null,
      dateOuverture: r.dateOuverture || null,
      dateFermeture: r.dateFermeture || null,
      source,
      updatedAt: new Date(),
    };

    if (existing) {
      await db.update(refEtablissement).set(payload).where(eq(refEtablissement.id, existing.id));
      updates += 1;
    } else {
      await db.insert(refEtablissement).values({
        codeRne: r.codeRne,
        ...payload,
      });
      inserts += 1;
    }
  }

  return { inserts, updates };
}

function collectEtablissementElements(xml: string): SiecleElement[] {
  const primary = extractSiecleElements(xml, "ETABLISSEMENT");
  if (primary.length) return primary;
  // Variantes rares / Communs imbriqués
  return [
    ...extractSiecleElements(xml, "UAJ"),
    ...extractSiecleElements(xml, "PARAMETRES"),
  ];
}

export async function importSiecleEtablissementsXml(
  etablissementId: string,
  filename: string,
  xml: string,
  opts?: { cycle?: SiecleImportCycle },
): Promise<{ inserts: number; updates: number; rows: number; message: string }> {
  const batchSize = 500;
  let batch: RefEtablissementRow[] = [];
  let totalRows = 0;
  let inserts = 0;
  let updates = 0;
  let skippedNoRne = 0;
  const cycle = opts?.cycle;
  const cycleNote = cycle ? ` · ${siecleCycleLabel(cycle)}` : "";

  const flush = async () => {
    if (!batch.length) return;
    const result = await upsertRefEtablissementRows(batch);
    inserts += result.inserts;
    updates += result.updates;
    batch = [];
  };

  const elements = collectEtablissementElements(xml);
  for (const el of elements) {
    const row = parseEtablissementElement(el);
    if (!row) {
      skippedNoRne += 1;
      continue;
    }
    batch.push(row);
    totalRows += 1;
    if (batch.length >= batchSize) await flush();
  }

  await flush();

  if (!totalRows) {
    throw new Error(
      `Aucun établissement avec CODE_RNE / UAI lisible dans ${filename} ` +
        `(${elements.length} bloc(s) ETABLISSEMENT/UAJ trouvé(s), ${skippedNoRne} sans code). ` +
        `Vérifiez que le fichier est bien Etablissements.xml dézippé (BEE_ETABLISSEMENTS).`,
    );
  }

  const db = getDb();
  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: filename,
    statut: "ok",
    nbInserts: inserts,
    nbUpdates: updates,
    rapportJson: {
      kind: "etablissements",
      ...(cycle ? { cycle } : {}),
      rows: totalRows,
      blocks: elements.length,
      skippedNoRne,
    },
  });

  return {
    inserts,
    updates,
    rows: totalRows,
    message: `${filename} (etablissements${cycleNote}) : ${totalRows} établissements — ${inserts} créés, ${updates} mis à jour.`,
  };
}
