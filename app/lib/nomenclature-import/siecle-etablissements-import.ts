import "server-only";

import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refEtablissement } from "@/db/schema";

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

function tagValue(block: string, tag: string): string {
  const m = block.match(new RegExp(`<${tag}[^>]*>([^<]*)</${tag}>`, "i"));
  return (m?.[1] ?? "").trim();
}

function parseSiecleDate(raw: string): string | null {
  const v = raw.trim();
  if (!v) return null;
  if (/^\d{4}-\d{2}-\d{2}$/.test(v)) return v;
  const fr = v.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  if (fr) return `${fr[3]}-${fr[2]}-${fr[1]}`;
  return null;
}

function* iterateXmlBlocks(xml: string, tag: string): Generator<string> {
  const open = `<${tag}`;
  const close = `</${tag}>`;
  let pos = 0;
  while (pos < xml.length) {
    const start = xml.indexOf(open, pos);
    if (start === -1) break;
    const innerStart = xml.indexOf(">", start);
    if (innerStart === -1) break;
    const end = xml.indexOf(close, innerStart);
    if (end === -1) break;
    yield xml.slice(innerStart + 1, end);
    pos = end + close.length;
  }
}

export function parseEtablissementBlock(block: string): RefEtablissementRow | null {
  const codeRne =
    tagValue(block, "CODE_RNE") ||
    tagValue(block, "CODE_ETABLISSEMENT") ||
    tagValue(block, "RNE");
  if (!codeRne) return null;

  const adresseParts = [
    tagValue(block, "ADRESSE1"),
    tagValue(block, "ADRESSE2"),
    tagValue(block, "CODE_POSTAL"),
    tagValue(block, "COMMUNE"),
    tagValue(block, "VILLE"),
  ].filter(Boolean);

  return {
    codeRne,
    codeNature: tagValue(block, "CODE_NATURE") || tagValue(block, "NATURE") || undefined,
    codeType: tagValue(block, "CODE_TYPE") || tagValue(block, "TYPE") || undefined,
    codeSecteur: tagValue(block, "CODE_SECTEUR") || tagValue(block, "SECTEUR") || undefined,
    sigle: tagValue(block, "SIGLE") || undefined,
    denomPrinc:
      tagValue(block, "DENOM_PRINC") ||
      tagValue(block, "DENOMINATION_PRINCIPALE") ||
      tagValue(block, "NOM") ||
      undefined,
    denomCompl:
      tagValue(block, "DENOM_COMPL") ||
      tagValue(block, "DENOMINATION_COMPLEMENTAIRE") ||
      undefined,
    adresse: adresseParts.length ? adresseParts.join(", ") : undefined,
    dateOuverture: parseSiecleDate(tagValue(block, "DATE_OUVERTURE")),
    dateFermeture: parseSiecleDate(tagValue(block, "DATE_FERMETURE")),
  };
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

export async function importSiecleEtablissementsXml(
  etablissementId: string,
  filename: string,
  xml: string,
): Promise<{ inserts: number; updates: number; rows: number; message: string }> {
  const batchSize = 500;
  let batch: RefEtablissementRow[] = [];
  let totalRows = 0;
  let inserts = 0;
  let updates = 0;

  const flush = async () => {
    if (!batch.length) return;
    const result = await upsertRefEtablissementRows(batch);
    inserts += result.inserts;
    updates += result.updates;
    batch = [];
  };

  for (const block of iterateXmlBlocks(xml, "ETABLISSEMENT")) {
    const row = parseEtablissementBlock(block);
    if (!row) continue;
    batch.push(row);
    totalRows += 1;
    if (batch.length >= batchSize) await flush();
  }

  await flush();

  const db = getDb();
  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: filename,
    statut: "ok",
    nbInserts: inserts,
    nbUpdates: updates,
    rapportJson: { kind: "etablissements", rows: totalRows },
  });

  return {
    inserts,
    updates,
    rows: totalRows,
    message: `${filename} (etablissements) : ${totalRows} établissements — ${inserts} créés, ${updates} mis à jour.`,
  };
}
