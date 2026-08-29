import type { NomenclatureUpsertRow } from "@/app/lib/nomenclature-import/siecle-xml-types";
import {
  codeFromElement,
  extractSiecleElements,
  libelleFromBlock,
  parseSiecleDate,
  tagValue,
  type SiecleElement,
} from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";
import { inferSecteurFromMef } from "@/app/lib/mef-secteur-inference";

type SectionSpec = {
  tag: string;
  type: string;
  codeAttrs: string[];
  codeTags?: string[];
  libelleTags?: string[];
  metadataTags?: string[];
};

type StructurePole = "COLLÈGE" | "LYCÉE" | "ÉCOLE";

const SIMPLE_SECTIONS: SectionSpec[] = [
  { tag: "MEF", type: "mef", codeAttrs: ["CODE_MEF"] },
  { tag: "MATIERE", type: "matiere", codeAttrs: ["CODE_MATIERE"] },
  { tag: "REGIME", type: "regime", codeAttrs: ["CODE_REGIME"] },
  { tag: "STATUT_ELEVE", type: "statut_eleve", codeAttrs: ["CODE_STATUT"] },
  {
    tag: "MOTIF_SORTIE",
    type: "motif_sortie",
    codeAttrs: ["CODE_MOTIF_SORTIE", "CODE_MOTIF"],
  },
  { tag: "LIEN_PARENTE", type: "lien_parente", codeAttrs: ["CODE_PARENTE"] },
  { tag: "BOURSE", type: "bourse", codeAttrs: ["CODE_BOURSE"] },
  {
    tag: "PROFESSION",
    type: "csp",
    codeAttrs: ["CODE_PROFESSION"],
    libelleTags: ["LIBELLE_LONG_PCS", "LIBELLE_LONG", "LIBELLE_COURT_PCS", "LIBELLE_COURT"],
  },
  { tag: "PROVENANCE", type: "provenance", codeAttrs: ["CODE_PROVENANCE"] },
  { tag: "CIVILITE", type: "civilite", codeAttrs: ["CODE_CIVILITE"] },
  { tag: "CONTRAT", type: "contrat", codeAttrs: ["CODE_CONTRAT"] },
  { tag: "TYPE_ETABLISSEMENT", type: "type_etablissement", codeAttrs: ["CODE_TYPE_ETAB"] },
  {
    tag: "MODALITE_ELECTION",
    type: "modalite_election",
    codeAttrs: ["CODE_MODALITE_ELECT"],
  },
  {
    tag: "MODALITE_COURS",
    type: "modalite_cours",
    codeAttrs: ["CODE_MODALITE_COURS"],
  },
];

function pushSimpleSection(rows: NomenclatureUpsertRow[], xml: string, spec: SectionSpec): void {
  for (const el of extractSiecleElements(xml, spec.tag)) {
    const code = codeFromElement(el, spec.codeAttrs, spec.codeTags);
    if (!code) continue;
    const libelle = libelleFromBlock(el.inner, code, spec.libelleTags);
    const metadataJson: Record<string, unknown> = {};

    if (spec.type === "mef") {
      const formation = tagValue(el.inner, "FORMATION");
      const codeMefstat = tagValue(el.inner, "CODE_MEFSTAT");
      const mefRattachement = tagValue(el.inner, "MEF_RATTACHEMENT");
      const inscriptionEtab = tagValue(el.inner, "INSCRIPTION_ETAB");
      if (formation) metadataJson.formation = formation;
      if (codeMefstat) metadataJson.codeMefstat = codeMefstat;
      if (mefRattachement) metadataJson.mefRattachement = mefRattachement;
      if (inscriptionEtab) metadataJson.inscriptionEtab = inscriptionEtab;
      const validFrom = parseSiecleDate(tagValue(el.inner, "DATE_OUVERTURE"));
      const validTo = parseSiecleDate(tagValue(el.inner, "DATE_FERMETURE"));
      rows.push({
        type: spec.type,
        code,
        libelleCourt: tagValue(el.inner, "LIBELLE_COURT") || libelle,
        libelleLong: libelle,
        metadataJson: Object.keys(metadataJson).length ? metadataJson : undefined,
        validFrom,
        validTo,
      });
      continue;
    }

    if (spec.type === "matiere") {
      const codeGestion = tagValue(el.inner, "CODE_GESTION");
      if (codeGestion) metadataJson.codeGestion = codeGestion;
      const typeLv = tagValue(el.inner, "TYPE_LANGUE_VIVANTE");
      if (typeLv) metadataJson.typeLangueVivante = typeLv;
    }

    for (const t of spec.metadataTags || []) {
      const v = tagValue(el.inner, t);
      if (v) metadataJson[t.toLowerCase()] = v;
    }

    rows.push({
      type: spec.type,
      code,
      libelleCourt: tagValue(el.inner, "LIBELLE_COURT") || libelle,
      libelleLong: libelle,
      metadataJson: Object.keys(metadataJson).length ? metadataJson : undefined,
    });
  }
}

function pushProgrammes(rows: NomenclatureUpsertRow[], xml: string): void {
  for (const el of extractSiecleElements(xml, "PROGRAMME")) {
    const codeMef = tagValue(el.inner, "CODE_MEF");
    const codeMatiere = tagValue(el.inner, "CODE_MATIERE");
    if (!codeMef || !codeMatiere) continue;
    const code = `${codeMef}:${codeMatiere}`;
    const modalite = tagValue(el.inner, "CODE_MODALITE_ELECT");
    const horaire = tagValue(el.inner, "HORAIRE");
    rows.push({
      type: "programme",
      code,
      libelleCourt: `${codeMatiere} (${codeMef})`,
      libelleLong: `Programme ${codeMatiere} — MEF ${codeMef}`,
      metadataJson: {
        codeMef,
        codeMatiere,
        ...(modalite ? { codeModaliteElect: modalite } : {}),
        ...(horaire ? { horaire } : {}),
      },
    });
  }
}

function pushOptionsObligatoires(rows: NomenclatureUpsertRow[], xml: string): void {
  for (const el of extractSiecleElements(xml, "OPTION_OBLIGATOIRE")) {
    const codeMef = tagValue(el.inner, "CODE_MEF");
    const codeMatiere = tagValue(el.inner, "CODE_MATIERE");
    if (!codeMef || !codeMatiere) continue;
    const rang = tagValue(el.inner, "RANG_OPTION") || "0";
    const code = `${codeMef}:${codeMatiere}:${rang}`;
    rows.push({
      type: "option",
      code,
      libelleCourt: `${codeMatiere} opt.${rang}`,
      libelleLong: `Option ${codeMatiere} — MEF ${codeMef} (rang ${rang})`,
      metadataJson: { codeMef, codeMatiere, rangOption: rang },
    });
  }
}

function extractMefsFromStructureBlock(inner: string): string[] {
  const mefs: string[] = [];
  for (const mefBlock of extractSiecleElements(inner, "MEF_APPARTENANCE")) {
    const mefCode = tagValue(mefBlock.inner, "CODE_MEF");
    if (mefCode) mefs.push(mefCode);
  }
  const directMef = tagValue(inner, "CODE_MEF");
  if (directMef) mefs.push(directMef);
  return mefs;
}

function inferStructurePole(code: string, libelle: string, mefs: string[]): StructurePole | undefined {
  for (const mef of mefs) {
    const secteur = inferSecteurFromMef(mef, mef);
    if (secteur === "college") return "COLLÈGE";
    if (secteur === "lycee") return "LYCÉE";
    if (secteur === "ecole") return "ÉCOLE";
  }

  const trimmedCode = code.trim();
  const blob = `${trimmedCode} ${libelle}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase();

  if (
    /\b(TPS|PS[^A-Z]|PSA|PSB|MSA|MSB|GSA|GSB|MATERNELLE)\b/.test(blob) ||
    /^(TPS|PS|MS|GS)/.test(trimmedCode)
  ) {
    return "ÉCOLE";
  }
  if (
    /\b(CP|CE1|CE2|CM1|CM2|PRIMAIRE|ELEMENTAIRE|ÉLÉMENTAIRE)\b/.test(blob) ||
    /^(CP|CE1|CE2|CM1|CM2)/.test(trimmedCode)
  ) {
    return "ÉCOLE";
  }
  if (
    /\b(COLLEGE|COLLÈGE|6EME|6ÈME|5EME|5ÈME|4EME|4ÈME|3EME|3ÈME)\b/.test(blob) ||
    /^[3456][\s./-]?[A-Z0-9]*$/i.test(trimmedCode)
  ) {
    return "COLLÈGE";
  }
  if (
    /\b(LYCEE|LYCÉE|TERMINALE|TLE|PREMIERE|PREMIÈRE|1ERE|1ÈRE|1RE|2NDE|SECONDE|CAP|BTS)\b/.test(
      blob,
    ) ||
    /^1[\s./-]?[A-Z0-9]*$/i.test(trimmedCode) ||
    /^2[\s./-]?[A-Z0-9]*$/i.test(trimmedCode) ||
    /^T[A-Z0-9]/i.test(trimmedCode)
  ) {
    return "LYCÉE";
  }

  return undefined;
}

function buildDivisionRow(
  el: SiecleElement,
  sourceTag: "DIVISION" | "STRUCTURE" | "GROUPE",
): NomenclatureUpsertRow | null {
  const code = codeFromElement(el, ["CODE_STRUCTURE", "CODE_DIVISION", "CODE_GROUPE"]);
  if (!code) return null;

  const libelle = libelleFromBlock(el.inner, code);
  const mefs = extractMefsFromStructureBlock(el.inner);
  const pole = inferStructurePole(code, libelle, mefs);

  return {
    type: "division",
    code,
    libelleCourt: libelle,
    libelleLong: libelle,
    metadataJson: {
      sourceTag,
      codeContrat: tagValue(el.inner, "CODE_CONTRAT") || undefined,
      codeRne: tagValue(el.inner, "CODE_RNE") || undefined,
      mefs: mefs.length ? mefs : undefined,
      pole,
    },
  };
}

/** Parse BEE_NOMENCLATURES v5 (codes en attribut XML). */
export function parseNomenclatureXml(xml: string): NomenclatureUpsertRow[] {
  const rows: NomenclatureUpsertRow[] = [];

  for (const spec of SIMPLE_SECTIONS) {
    pushSimpleSection(rows, xml, spec);
  }
  pushProgrammes(rows, xml);
  pushOptionsObligatoires(rows, xml);

  return rows;
}

/** Parse BEE_STRUCTURES — divisions (classes) et métadonnées MEF. */
export function parseStructuresDivisions(xml: string): NomenclatureUpsertRow[] {
  const rows: NomenclatureUpsertRow[] = [];
  const seen = new Set<string>();

  const push = (row: NomenclatureUpsertRow) => {
    const key = row.code.trim().toUpperCase();
    if (!key || seen.has(key)) return;
    seen.add(key);
    rows.push(row);
  };

  for (const tag of ["DIVISION", "STRUCTURE"] as const) {
    for (const el of extractSiecleElements(xml, tag)) {
      const row = buildDivisionRow(el, tag);
      if (row) push(row);
    }
  }

  if (!rows.length) {
    for (const el of extractSiecleElements(xml, "GROUPE")) {
      const row = buildDivisionRow(el, "GROUPE");
      if (row) push(row);
    }
  }

  return rows;
}

/** Groupes pédagogiques Siècle (Structures.xml → balise GROUPE). */
export function parseStructuresGroupes(xml: string): { code: string; libelle: string }[] {
  const rows: { code: string; libelle: string }[] = [];
  for (const el of extractSiecleElements(xml, "GROUPE")) {
    const code = codeFromElement(el, ["CODE_GROUPE", "CODE_STRUCTURE", "CODE_DIVISION"]);
    if (!code) continue;
    const libelle = libelleFromBlock(el.inner, code);
    rows.push({ code, libelle });
  }
  return rows;
}
