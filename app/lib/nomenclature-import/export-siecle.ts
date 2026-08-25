import "server-only";

import { and, asc, desc, eq, inArray } from "drizzle-orm";
import JSZip from "jszip";
import { getDb } from "@/db/index";
import {
  anneeScolaire,
  eleve,
  eleveFoyerLink,
  eleveScolarite,
  etablissement,
  foyer,
  foyerResponsable,
  nomenclatureImportLog,
  refEtablissement,
  refNomenclature,
} from "@/db/schema";
import { currentSchoolYearLabel } from "@/app/lib/ent-core-db";
import { getJson } from "@/app/lib/s3-storage";

const SIECLE_ELEVE_MAP_KEY = "siecle/eleve-id-map.json";
const EXPORT_LOGICIEL = "SCOLIA";

export type SiecleExportResult = {
  filename: string;
  zipBuffer: Buffer;
  xmlFilename: string;
  stats: {
    eleves: number;
    personnes: number;
    liens: number;
    sansIne: number;
    sansSiecleId: number;
  };
  numEnvoi: string;
  uaj: string;
  anneeScolaire: string;
};

type SiecleEleveIdMap = Record<string, string>;

type ExportPersonne = {
  idPrv: string;
  nom: string;
  prenom: string;
  email: string;
  telephone: string;
  adresse: string;
  codePostal: string;
  ville: string;
};

type ExportEleve = {
  idPrv: string;
  idSiecle: string;
  ine: string;
  nom: string;
  prenom: string;
  sexe: string;
  dateNaissance: string;
  codeRegime: string;
  codeMef: string;
  codeDivision: string;
  codeStatut: string;
  responsables: Array<{
    idPrvPer: string;
    codeParente: string;
    payeur: boolean;
    heberge: boolean;
    contactPrioritaire: boolean;
  }>;
};

function xmlEscape(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function toPrvId(uuid: string): string {
  return uuid.replace(/-/g, "").slice(0, 16).toUpperCase();
}

function formatSiecleDate(iso: string | null | undefined): string {
  if (!iso) return "";
  const m = String(iso).match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (!m) return "";
  return `${m[3]}/${m[2]}/${m[1]}`;
}

function formatExportTimestamp(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    String(now.getFullYear()) +
    pad(now.getMonth() + 1) +
    pad(now.getDate()) +
    pad(now.getHours()) +
    pad(now.getMinutes()) +
    pad(now.getSeconds())
  );
}

function boolXml(v: boolean): string {
  return v ? "1" : "0";
}

function regimeToCode(raw: string | null | undefined): string {
  const s = String(raw ?? "").trim();
  if (!s) return "0";
  if (s === "0" || s === "1" || s === "2" || s === "3") return s;
  const lower = s.toLowerCase();
  if (lower.includes("interne")) return "2";
  if (lower.includes("demi") || lower.includes("dp")) return "1";
  return "0";
}

function sexeToCode(sexe: string | null | undefined): string {
  if (sexe === "F") return "2";
  if (sexe === "M") return "1";
  return "";
}

function parenteCodeFromRelation(relation: string): string {
  const m = relation.match(/parente:(\d+)/i);
  if (m) return m[1];
  const lower = relation.toLowerCase();
  if (lower.includes("mere") || lower.includes("mère")) return "2";
  if (lower.includes("pere") || lower.includes("père")) return "1";
  return "9";
}

async function loadSiecleEleveIdMap(): Promise<SiecleEleveIdMap> {
  const hit = await getJson<SiecleEleveIdMap>(SIECLE_ELEVE_MAP_KEY);
  if (!hit?.data || typeof hit.data !== "object") return {};
  return hit.data;
}

function invertEleveIdMap(map: SiecleEleveIdMap): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [siecleId, ine] of Object.entries(map)) {
    const key = ine.trim().toUpperCase();
    if (key) out[key] = siecleId;
  }
  return out;
}

async function resolveUaj(etablissementId: string): Promise<string> {
  const envUaj = process.env.SIECLE_UAJ?.trim();
  if (envUaj) return envUaj;

  const db = getDb();
  const [fromRef] = await db
    .select({ codeRne: refEtablissement.codeRne })
    .from(refEtablissement)
    .orderBy(desc(refEtablissement.updatedAt))
    .limit(1);
  if (fromRef?.codeRne) return fromRef.codeRne;

  const [tenant] = await db
    .select({ slug: etablissement.slug })
    .from(etablissement)
    .where(eq(etablissement.id, etablissementId))
    .limit(1);
  return tenant?.slug?.toUpperCase().slice(0, 8) || "0000000X";
}

async function resolveAnneeScolaireLabel(etablissementId: string): Promise<string> {
  const db = getDb();
  const [current] = await db
    .select({ label: anneeScolaire.label })
    .from(anneeScolaire)
    .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.isCurrent, true)))
    .limit(1);
  return current?.label || currentSchoolYearLabel();
}

async function buildDivisionCodeMap(etablissementId: string): Promise<Map<string, string>> {
  const db = getDb();
  const rows = await db
    .select({
      code: refNomenclature.code,
      libelleCourt: refNomenclature.libelleCourt,
      libelleLong: refNomenclature.libelleLong,
    })
    .from(refNomenclature)
    .where(
      and(eq(refNomenclature.etablissementId, etablissementId), eq(refNomenclature.type, "division")),
    );

  const map = new Map<string, string>();
  for (const r of rows) {
    map.set(r.code.toLowerCase(), r.code);
    if (r.libelleCourt) map.set(r.libelleCourt.toLowerCase(), r.code);
    if (r.libelleLong) map.set(r.libelleLong.toLowerCase(), r.code);
  }
  return map;
}

function resolveDivision(classe: string | null | undefined, divisions: Map<string, string>): string {
  const raw = String(classe ?? "").trim();
  if (!raw) return "";
  const hit = divisions.get(raw.toLowerCase());
  return hit || raw;
}

async function nextNumEnvoi(etablissementId: string): Promise<string> {
  const db = getDb();
  const logs = await db
    .select({ rapportJson: nomenclatureImportLog.rapportJson })
    .from(nomenclatureImportLog)
    .where(eq(nomenclatureImportLog.etablissementId, etablissementId))
    .orderBy(desc(nomenclatureImportLog.dateImport))
    .limit(50);

  let max = 0;
  for (const log of logs) {
    const rapport = log.rapportJson as { sens?: string; numEnvoi?: string } | null;
    if (rapport?.sens !== "export") continue;
    const n = Number(rapport.numEnvoi);
    if (Number.isFinite(n) && n > max) max = n;
  }
  return String(max + 1).padStart(3, "0");
}

function renderPersonneXml(p: ExportPersonne): string {
  return `<PERSONNE>
  <ID_PRV_PER>${xmlEscape(p.idPrv)}</ID_PRV_PER>
  <NOM>${xmlEscape(p.nom)}</NOM>
  <PRENOM>${xmlEscape(p.prenom)}</PRENOM>
  <MEL>${xmlEscape(p.email)}</MEL>
  <TEL>${xmlEscape(p.telephone)}</TEL>
  <ADRESSE_1>${xmlEscape(p.adresse)}</ADRESSE_1>
  <CODE_POSTAL>${xmlEscape(p.codePostal)}</CODE_POSTAL>
  <VILLE>${xmlEscape(p.ville)}</VILLE>
</PERSONNE>`;
}

function renderEleveXml(e: ExportEleve): string {
  const responsables = e.responsables
    .map(
      (r) => `<LEGAL>
  <ID_PRV_PER>${xmlEscape(r.idPrvPer)}</ID_PRV_PER>
  <CODE_PARENTE>${xmlEscape(r.codeParente)}</CODE_PARENTE>
  <PAIE_FRAIS_SCOLAIRES>${boolXml(r.payeur)}</PAIE_FRAIS_SCOLAIRES>
  <HEBERGE_ELEVE>${boolXml(r.heberge)}</HEBERGE_ELEVE>
  <A_CONTACTER_EN_PRIORITE>${boolXml(r.contactPrioritaire)}</A_CONTACTER_EN_PRIORITE>
</LEGAL>`,
    )
    .join("\n");

  return `<ELEVE>
  <ID_PRV_ELE>${xmlEscape(e.idPrv)}</ID_PRV_ELE>
  <ID_SIECLE_ELE>${xmlEscape(e.idSiecle)}</ID_SIECLE_ELE>
  <ID_NATIONAL>${xmlEscape(e.ine)}</ID_NATIONAL>
  <NOM_DE_FAMILLE>${xmlEscape(e.nom)}</NOM_DE_FAMILLE>
  <PRENOM>${xmlEscape(e.prenom)}</PRENOM>
  <CODE_SEXE>${xmlEscape(e.sexe)}</CODE_SEXE>
  <DATE_NAISS>${xmlEscape(e.dateNaissance)}</DATE_NAISS>
  <CODE_REGIME>${xmlEscape(e.codeRegime)}</CODE_REGIME>
  <RESPONSABLES_ELEVE>
${responsables}
  </RESPONSABLES_ELEVE>
  <SCOLARITE_ACTIVE>
    <CODE_MEF>${xmlEscape(e.codeMef)}</CODE_MEF>
    <CODE_DIVISION>${xmlEscape(e.codeDivision)}</CODE_DIVISION>
    <CODE_STATUT>${xmlEscape(e.codeStatut)}</CODE_STATUT>
  </SCOLARITE_ACTIVE>
</ELEVE>`;
}

function buildImportElevesXml(input: {
  uaj: string;
  anneeScolaire: string;
  dateImport: string;
  numEnvoi: string;
  personnes: ExportPersonne[];
  eleves: ExportEleve[];
}): string {
  const personnesXml = input.personnes.map(renderPersonneXml).join("\n");
  const elevesXml = input.eleves.map(renderEleveXml).join("\n");

  return `<?xml version="1.0" encoding="ISO-8859-15"?>
<IMPORT_ELEVES VERSION="4.0">
<PARAMETRES>
  <UAJ>${xmlEscape(input.uaj)}</UAJ>
  <ANNEE_SCOLAIRE>${xmlEscape(input.anneeScolaire)}</ANNEE_SCOLAIRE>
  <DATE_IMPORT>${xmlEscape(input.dateImport)}</DATE_IMPORT>
  <NUM_ENVOI>${xmlEscape(input.numEnvoi)}</NUM_ENVOI>
  <LOGICIEL>${EXPORT_LOGICIEL}</LOGICIEL>
</PARAMETRES>
<DONNEES>
  <PERSONNES>
${personnesXml}
  </PERSONNES>
  <ELEVES>
${elevesXml}
  </ELEVES>
</DONNEES>
</IMPORT_ELEVES>`;
}

export async function buildSiecleExportBundle(
  etablissementId: string,
): Promise<SiecleExportResult> {
  const db = getDb();
  const [eleveRows, siecleMap, divisions, uaj, anneeLabel, numEnvoi] = await Promise.all([
    db
      .select()
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.status, "inscrit")))
      .orderBy(asc(eleve.nom), asc(eleve.prenom)),
    loadSiecleEleveIdMap(),
    buildDivisionCodeMap(etablissementId),
    resolveUaj(etablissementId),
    resolveAnneeScolaireLabel(etablissementId),
    nextNumEnvoi(etablissementId),
  ]);

  if (!eleveRows.length) {
    throw new Error("Aucun élève inscrit à exporter.");
  }

  const ineToSiecle = invertEleveIdMap(siecleMap);
  const eleveIds = eleveRows.map((e) => e.id);

  const links = await db
    .select({
      eleveId: eleveFoyerLink.eleveId,
      relation: eleveFoyerLink.relation,
      foyerId: eleveFoyerLink.foyerId,
    })
    .from(eleveFoyerLink)
    .where(
      and(
        eq(eleveFoyerLink.etablissementId, etablissementId),
        inArray(eleveFoyerLink.eleveId, eleveIds),
      ),
    );

  const foyerIds = [...new Set(links.map((l) => l.foyerId))];
  const [foyerRows, responsableRows] = foyerIds.length
    ? await Promise.all([
        db
          .select()
          .from(foyer)
          .where(
            and(eq(foyer.etablissementId, etablissementId), inArray(foyer.id, foyerIds)),
          ),
        db
          .select()
          .from(foyerResponsable)
          .where(
            and(
              eq(foyerResponsable.etablissementId, etablissementId),
              inArray(foyerResponsable.foyerId, foyerIds),
            ),
          )
          .orderBy(asc(foyerResponsable.rang)),
      ])
    : [[], []];

  const foyerById = new Map(foyerRows.map((f) => [f.id, f]));
  const responsablesByFoyer = new Map<string, typeof responsableRows>();
  for (const r of responsableRows) {
    const list = responsablesByFoyer.get(r.foyerId) || [];
    list.push(r);
    responsablesByFoyer.set(r.foyerId, list);
  }

  const scolariteRows = await db
    .select({ eleveId: eleveScolarite.eleveId, classe: eleveScolarite.classe })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, etablissementId),
        inArray(eleveScolarite.eleveId, eleveIds),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    );

  const scolariteClasse = new Map(scolariteRows.map((s) => [s.eleveId, s.classe]));

  const personnesMap = new Map<string, ExportPersonne>();
  const exportEleves: ExportEleve[] = [];
  let sansIne = 0;
  let sansSiecleId = 0;
  let liens = 0;

  for (const row of eleveRows) {
    const ine = (row.ine || "").trim().toUpperCase();
    if (!ine) sansIne += 1;
    const idSiecle = ine ? ineToSiecle[ine] || "" : "";
    if (ine && !idSiecle) sansSiecleId += 1;

    const classe = scolariteClasse.get(row.id) || row.classe;
    const responsables: ExportEleve["responsables"] = [];

    for (const link of links.filter((l) => l.eleveId === row.id)) {
      const respList = responsablesByFoyer.get(link.foyerId) || [];
      const foyerRow = foyerById.get(link.foyerId);
      for (const resp of respList) {
        const idPrvPer = toPrvId(resp.id);
        if (!personnesMap.has(idPrvPer)) {
          personnesMap.set(idPrvPer, {
            idPrv: idPrvPer,
            nom: resp.nom,
            prenom: resp.prenom,
            email: resp.email || "",
            telephone: resp.telephone || "",
            adresse: foyerRow?.adresse || "",
            codePostal: foyerRow?.codePostal || "",
            ville: foyerRow?.ville || "",
          });
        }
        responsables.push({
          idPrvPer,
          codeParente: parenteCodeFromRelation(link.relation),
          payeur: resp.payeur,
          heberge: false,
          contactPrioritaire: resp.contactUrgence,
        });
        liens += 1;
      }
    }

    exportEleves.push({
      idPrv: toPrvId(row.id),
      idSiecle,
      ine,
      nom: row.nom,
      prenom: row.prenom,
      sexe: sexeToCode(row.sexe),
      dateNaissance: formatSiecleDate(row.dateNaissance),
      codeRegime: regimeToCode(row.regime),
      codeMef: row.mef || "",
      codeDivision: resolveDivision(classe, divisions),
      codeStatut: row.status === "inscrit" ? "ST" : "ST",
      responsables,
    });
  }

  const now = new Date();
  const dateImport = formatExportTimestamp(now);
  const anneeCompact = anneeLabel.replace(/-/g, "").slice(2);
  const zipBase = `${uaj}PRIVE${anneeCompact}${dateImport}${numEnvoi}`;
  const xmlFilename = `${zipBase}.xml`;

  const xmlUtf8 = buildImportElevesXml({
    uaj,
    anneeScolaire: anneeLabel,
    dateImport,
    numEnvoi,
    personnes: [...personnesMap.values()],
    eleves: exportEleves,
  });
  const xmlBuffer = Buffer.from(xmlUtf8, "latin1");

  const zip = new JSZip();
  zip.file(xmlFilename, xmlBuffer);
  const zipBuffer = await zip.generateAsync({ type: "nodebuffer", compression: "DEFLATE" });

  return {
    filename: `${zipBase}.zip`,
    zipBuffer,
    xmlFilename,
    stats: {
      eleves: exportEleves.length,
      personnes: personnesMap.size,
      liens,
      sansIne,
      sansSiecleId,
    },
    numEnvoi,
    uaj,
    anneeScolaire: anneeLabel,
  };
}

export async function recordSiecleExportLog(
  etablissementId: string,
  bundle: SiecleExportResult,
): Promise<void> {
  const db = getDb();
  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: bundle.filename,
    source: "siecle_export",
    statut: "ok",
    nbInserts: bundle.stats.eleves,
    nbUpdates: 0,
    rapportJson: {
      sens: "export",
      numEnvoi: bundle.numEnvoi,
      uaj: bundle.uaj,
      anneeScolaire: bundle.anneeScolaire,
      stats: bundle.stats,
      xmlFilename: bundle.xmlFilename,
    },
  });
}
