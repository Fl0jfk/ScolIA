import type { NomenclatureUpsertRow } from "@/app/lib/nomenclature-import/siecle-xml-types";
import {
  codeFromElement,
  extractSiecleElements,
  libelleFromBlock,
  parseSiecleDate,
  tagValue,
} from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";

function pushSimpleRows(
  rows: NomenclatureUpsertRow[],
  xml: string,
  tag: string,
  type: string,
  codeAttrs: string[],
): void {
  for (const el of extractSiecleElements(xml, tag)) {
    const code = codeFromElement(el, codeAttrs);
    if (!code) continue;
    const libelle = libelleFromBlock(el.inner, code);
    const validFrom = parseSiecleDate(tagValue(el.inner, "DATE_OUVERTURE"));
    const validTo = parseSiecleDate(tagValue(el.inner, "DATE_FERMETURE"));
    rows.push({
      type,
      code,
      libelleCourt: tagValue(el.inner, "LIBELLE_COURT") || libelle,
      libelleLong: libelle,
      metadataJson:
        validFrom || validTo
          ? { ...(validFrom ? { dateOuverture: validFrom } : {}), ...(validTo ? { dateFermeture: validTo } : {}) }
          : undefined,
    });
  }
}

/** Parse BEE_GEOGRAPHIQUE — pays, départements, communes. */
export function parseGeographiqueXml(xml: string): NomenclatureUpsertRow[] {
  const rows: NomenclatureUpsertRow[] = [];

  pushSimpleRows(rows, xml, "PAYS", "pays", ["CODE_PAYS", "CODE"]);
  pushSimpleRows(rows, xml, "PAYS_NATIONALITE", "pays", ["CODE_PAYS", "CODE"]);

  for (const el of extractSiecleElements(xml, "DEPARTEMENT")) {
    const code = codeFromElement(el, ["CODE_DEPARTEMENT_INSEE", "CODE_DEPARTEMENT", "CODE_DEPT", "CODE"]);
    if (!code) continue;
    const libelle = libelleFromBlock(el.inner, code);
    rows.push({
      type: "departement",
      code,
      libelleCourt: tagValue(el.inner, "LIBELLE_COURT") || libelle,
      libelleLong: libelle,
    });
  }

  for (const el of extractSiecleElements(xml, "COMMUNE")) {
    const code = codeFromElement(el, ["CODE_COMMUNE_INSEE", "CODE_INSEE", "CODE_COMMUNE", "CODE"]);
    if (!code) continue;
    const libelle = libelleFromBlock(el.inner, code, ["LIBELLE_LONG", "LIBELLE_COURT", "LIBELLE", "NOM"]);
    rows.push({
      type: "commune",
      code,
      libelleCourt: libelle,
      libelleLong: libelle,
      metadataJson: {
        codeDepartement:
          tagValue(el.inner, "CODE_DEPARTEMENT_INSEE") ||
          tagValue(el.inner, "CODE_DEPARTEMENT") ||
          tagValue(el.inner, "CODE_DEPT") ||
          undefined,
        dateOuverture: parseSiecleDate(tagValue(el.inner, "DATE_OUVERTURE")),
        dateFermeture: parseSiecleDate(tagValue(el.inner, "DATE_FERMETURE")),
      },
    });
  }

  return rows;
}
