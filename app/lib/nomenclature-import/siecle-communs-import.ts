import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { anneeScolaire, etablissement, nomenclatureImportLog } from "@/db/schema";
import {
  parseEtablissementBlock,
  upsertRefEtablissementRows,
  type RefEtablissementRow,
} from "@/app/lib/nomenclature-import/siecle-etablissements-import";

function extractBlocks(xml: string, tag: string): string[] {
  const re = new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)</${tag}>`, "gi");
  const out: string[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(xml))) {
    out.push(m[1] ?? "");
  }
  return out;
}

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

function normalizeAnneeLabel(raw: string): string {
  const v = raw.trim();
  if (/^\d{4}-\d{4}$/.test(v)) return v;
  const m = v.match(/^(\d{4})(\d{2})$/);
  if (m) return `${m[1]}-${Number(m[1]) + 1}`;
  return v;
}

function parseUajRow(block: string): RefEtablissementRow | null {
  const parsed = parseEtablissementBlock(block);
  if (parsed) return parsed;

  const codeRne = tagValue(block, "CODE_RNE") || tagValue(block, "UAJ");
  if (!codeRne) return null;

  return {
    codeRne,
    denomPrinc:
      tagValue(block, "NOM_ETABLISSEMENT") ||
      tagValue(block, "DENOM_PRINC") ||
      tagValue(block, "NOM") ||
      undefined,
    sigle: tagValue(block, "SIGLE") || undefined,
    adresse: [
      tagValue(block, "ADRESSE1"),
      tagValue(block, "ADRESSE2"),
      tagValue(block, "CODE_POSTAL"),
      tagValue(block, "COMMUNE"),
    ]
      .filter(Boolean)
      .join(", ") || undefined,
    codeNature: tagValue(block, "CODE_NATURE") || undefined,
    codeSecteur: tagValue(block, "CODE_SECTEUR") || undefined,
  };
}

export async function importSiecleCommunsXml(
  etablissementId: string,
  filename: string,
  xml: string,
): Promise<{ inserts: number; updates: number; rows: number; message: string }> {
  const db = getDb();
  let inserts = 0;
  let updates = 0;
  let rows = 0;
  const notes: string[] = [];

  const uajBlocks = [...extractBlocks(xml, "UAJ"), ...extractBlocks(xml, "PARAMETRES")];
  for (const block of uajBlocks) {
    const uaj = parseUajRow(block);
    if (!uaj?.codeRne) continue;
    rows += 1;
    const result = await upsertRefEtablissementRows([uaj]);
    inserts += result.inserts;
    updates += result.updates;

    const label = uaj.denomPrinc || uaj.sigle;
    if (label) {
      await db
        .update(etablissement)
        .set({ name: label, updatedAt: new Date() })
        .where(eq(etablissement.id, etablissementId));
      notes.push(`UAJ ${uaj.codeRne} — libellé établissement synchronisé.`);
    }
  }

  for (const block of extractBlocks(xml, "ANNEE_SCOLAIRE")) {
    const rawLabel =
      tagValue(block, "LIBELLE") ||
      tagValue(block, "ANNEE_SCOLAIRE") ||
      tagValue(block, "CODE");
    if (!rawLabel) continue;

    const label = normalizeAnneeLabel(rawLabel);
    const startsOn = parseSiecleDate(
      tagValue(block, "DATE_DEBUT") || tagValue(block, "DATE_DEBUT_ANNEE"),
    );
    const endsOn = parseSiecleDate(
      tagValue(block, "DATE_FIN") || tagValue(block, "DATE_FIN_ANNEE"),
    );
    rows += 1;

    const [existing] = await db
      .select({ id: anneeScolaire.id })
      .from(anneeScolaire)
      .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.label, label)))
      .limit(1);

    if (existing) {
      await db
        .update(anneeScolaire)
        .set({
          startsOn: startsOn || undefined,
          endsOn: endsOn || undefined,
          isCurrent: true,
          updatedAt: new Date(),
        })
        .where(eq(anneeScolaire.id, existing.id));
      updates += 1;
    } else {
      await db
        .update(anneeScolaire)
        .set({ isCurrent: false, updatedAt: new Date() })
        .where(eq(anneeScolaire.etablissementId, etablissementId));
      await db.insert(anneeScolaire).values({
        etablissementId,
        label,
        startsOn,
        endsOn,
        isCurrent: true,
      });
      inserts += 1;
    }
    notes.push(`Année scolaire ${label} enregistrée.`);
  }

  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: filename,
    statut: "ok",
    nbInserts: inserts,
    nbUpdates: updates,
    rapportJson: { kind: "communs", rows, notes },
  });

  return {
    inserts,
    updates,
    rows,
    message: `${filename} (communs) : ${rows} entrées — ${inserts} créées, ${updates} mises à jour.${notes.length ? ` ${notes.join(" ")}` : ""}`,
  };
}
