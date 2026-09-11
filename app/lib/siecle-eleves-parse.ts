import { buildEleveFolderName, type EleveConfig } from "@/app/lib/eleves-config";
import { isRegimeInterne } from "@/app/lib/eleve-regime";
import {
  attrValue,
  extractSiecleElements,
  firstNonEmpty,
  tagValue,
} from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";

function normalizeSiecleDate(raw: string): string {
  const s = raw.trim();
  const fr = s.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  if (fr) return `${fr[3]}-${fr[2]}-${fr[1]}`;
  const iso = s.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (iso) return `${iso[1]}-${iso[2]}-${iso[3]}`;
  return "";
}

/**
 * Map Siècle ELEVE_ID → INE (ID_NATIONAL) pour jointure Responsables.xml.
 * Siècle v5 met ELEVE_ID en **attribut** de `<ELEVE ELEVE_ID="…">` (parfois aussi en balise enfant).
 */
export function buildSiecleEleveIdToIneMap(xmlText: string): Record<string, string> {
  const map: Record<string, string> = {};
  for (const el of extractSiecleElements(xmlText, "ELEVE")) {
    const siecleId = firstNonEmpty(attrValue(el.attrs, "ELEVE_ID"), tagValue(el.inner, "ELEVE_ID"));
    const ine = firstNonEmpty(
      tagValue(el.inner, "ID_NATIONAL"),
      attrValue(el.attrs, "ID_NATIONAL"),
    ).toUpperCase();
    if (siecleId && ine) map[siecleId] = ine;
  }
  return map;
}

/** Parse ElevesSansAdresses.xml (BEE_ELEVES) → EleveConfig[] (regex, sans DOM). */
export function parseSiecleElevesXmlServer(xmlText: string): {
  eleves: EleveConfig[];
  internesCount: number;
  total: number;
  siecleEleveIdMap: Record<string, string>;
} {
  const eleves: EleveConfig[] = [];
  let internesCount = 0;

  for (const el of extractSiecleElements(xmlText, "ELEVE")) {
    const nom = firstNonEmpty(
      tagValue(el.inner, "NOM_DE_FAMILLE"),
      tagValue(el.inner, "NOM"),
      attrValue(el.attrs, "NOM_DE_FAMILLE"),
    );
    const prenom = firstNonEmpty(
      tagValue(el.inner, "PRENOM"),
      tagValue(el.inner, "PRENOM_1"),
      attrValue(el.attrs, "PRENOM"),
    );
    if (!nom || !prenom) continue;

    const ine = firstNonEmpty(
      tagValue(el.inner, "ID_NATIONAL"),
      attrValue(el.attrs, "ID_NATIONAL"),
    );
    const codeRegime = tagValue(el.inner, "CODE_REGIME");
    const codeSexe = tagValue(el.inner, "CODE_SEXE");
    const dateNaiss = tagValue(el.inner, "DATE_NAISS");
    const email = firstNonEmpty(tagValue(el.inner, "MEL"), tagValue(el.inner, "EMAIL"));
    const codeStructure = firstNonEmpty(
      tagValue(el.inner, "CODE_STRUCTURE"),
      tagValue(el.inner, "CODE_DIVISION"),
    );
    const codeMef = tagValue(el.inner, "CODE_MEF");
    const sexe: "M" | "F" | undefined =
      codeSexe === "2" ? "F" : codeSexe === "1" ? "M" : undefined;
    const dateNaissance = normalizeSiecleDate(dateNaiss);
    const folderName = buildEleveFolderName(nom, prenom);

    if (isRegimeInterne(codeRegime)) internesCount += 1;

    eleves.push({
      ine,
      nom,
      prenom,
      folderName,
      ...(codeStructure ? { classe: codeStructure } : {}),
      ...(codeMef ? { mef: codeMef } : {}),
      ...(email ? { email } : {}),
      ...(dateNaissance ? { dateNaissance } : {}),
      ...(codeRegime ? { regime: codeRegime } : {}),
      ...(sexe ? { sexe } : {}),
    });
  }

  return {
    eleves,
    internesCount,
    total: eleves.length,
    siecleEleveIdMap: buildSiecleEleveIdToIneMap(xmlText),
  };
}
