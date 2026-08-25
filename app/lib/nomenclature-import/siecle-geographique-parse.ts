import type { NomenclatureUpsertRow } from "@/app/lib/nomenclature-import/siecle-xml-types";

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

function pushRows(
  rows: NomenclatureUpsertRow[],
  blocks: string[],
  type: string,
  codeTags: string[],
  libelleTags: string[],
  metadataTags?: string[],
): void {
  for (const block of blocks) {
    const code = codeTags.map((t) => tagValue(block, t)).find((v) => v);
    if (!code) continue;
    const libelle =
      libelleTags.map((t) => tagValue(block, t)).find((v) => v) || code;
    const metadataJson: Record<string, unknown> = {};
    for (const t of metadataTags || []) {
      const v = tagValue(block, t);
      if (v) metadataJson[t.toLowerCase()] = v;
    }
    rows.push({
      type,
      code,
      libelleCourt: libelle,
      libelleLong: libelle,
      metadataJson: Object.keys(metadataJson).length ? metadataJson : undefined,
    });
  }
}

/** Parse BEE_GEOGRAPHIQUE — pays, départements, communes. */
export function parseGeographiqueXml(xml: string): NomenclatureUpsertRow[] {
  const rows: NomenclatureUpsertRow[] = [];

  pushRows(
    rows,
    extractBlocks(xml, "PAYS"),
    "pays",
    ["CODE_PAYS", "CODE"],
    ["LIBELLE_LONG", "LIBELLE_COURT", "LIBELLE", "NOM"],
  );
  pushRows(
    rows,
    extractBlocks(xml, "PAYS_NATIONALITE"),
    "pays",
    ["CODE_PAYS", "CODE"],
    ["LIBELLE_LONG", "LIBELLE_COURT", "LIBELLE", "NOM"],
  );

  pushRows(
    rows,
    extractBlocks(xml, "DEPARTEMENT"),
    "departement",
    ["CODE_DEPARTEMENT", "CODE_DEPT", "CODE"],
    ["LIBELLE_LONG", "LIBELLE_COURT", "LIBELLE", "NOM"],
  );

  for (const block of extractBlocks(xml, "COMMUNE")) {
    const code =
      tagValue(block, "CODE_INSEE") ||
      tagValue(block, "CODE_COMMUNE") ||
      tagValue(block, "CODE");
    if (!code) continue;
    const libelle =
      tagValue(block, "LIBELLE_LONG") ||
      tagValue(block, "LIBELLE_COURT") ||
      tagValue(block, "LIBELLE") ||
      tagValue(block, "NOM") ||
      code;
    rows.push({
      type: "commune",
      code,
      libelleCourt: libelle,
      libelleLong: libelle,
      metadataJson: {
        codeDepartement:
          tagValue(block, "CODE_DEPARTEMENT") ||
          tagValue(block, "CODE_DEPT") ||
          undefined,
        dateOuverture: tagValue(block, "DATE_OUVERTURE") || undefined,
        dateFermeture: tagValue(block, "DATE_FERMETURE") || undefined,
      },
    });
  }

  return rows;
}
