import * as XLSX from "xlsx";
import type { EnseignantConfig } from "@/app/lib/enseignants-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type EnseignantsImportResult =
  | {
      ok: true;
      enseignants: EnseignantConfig[];
      skipped: string[];
      warnings: string[];
    }
  | { ok: false; error: string };

export type EnseignantsMergeStats = {
  total: number;
  added: number;
  updated: number;
  kept: number;
};

function normHeader(v: unknown): string {
  return String(v ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function findCol(headers: string[], aliases: string[]): number {
  for (let i = 0; i < headers.length; i++) {
    const h = headers[i]!;
    if (aliases.some((a) => h === a || h.includes(a))) return i;
  }
  return -1;
}

function normEmail(raw: string): string | undefined {
  const e = raw.trim().toLowerCase();
  return e && e.includes("@") ? e : undefined;
}

function findColExclusive(headers: string[], aliases: string[], used: Set<number>): number {
  for (let i = 0; i < headers.length; i++) {
    if (used.has(i)) continue;
    const h = headers[i]!;
    if (aliases.some((a) => h === a || h.includes(a))) {
      used.add(i);
      return i;
    }
  }
  return -1;
}

function resolveEmailColumns(headers: string[]): { perso: number; pro: number } {
  const used = new Set<number>();
  const pro = findColExclusive(headers, [
    "email pro",
    "mail pro",
    "email professionnel",
    "mail professionnel",
    "courriel pro",
    "email etablissement",
    "email établissement",
    "mail etablissement",
  ], used);
  const perso = findColExclusive(headers, [
    "email personnel",
    "mail personnel",
    "email perso",
    "mail perso",
    "courriel personnel",
  ], used);
  let generic = -1;
  if (perso < 0 && pro < 0) {
    generic = findCol(headers, ["email", "e-mail", "mail", "courriel"]);
  }
  return {
    perso: perso >= 0 ? perso : generic,
    pro,
  };
}

function cellStr(row: unknown[], idx: number): string {
  if (idx < 0) return "";
  return String(row[idx] ?? "").trim();
}

function parseSecteurLabel(raw: string): Secteur | null {
  const s = normHeader(raw);
  if (!s) return null;
  if (s.includes("ecole") || s.includes("primaire") || s.includes("maternelle")) return "ecole";
  if (s.includes("college") || s.includes("collège")) return "college";
  if (s.includes("lycee") || s.includes("lycée")) return "lycee";
  return null;
}

/** Déduit école / collège / lycée à partir d'une cellule « liste des classes » (Charlemagne). */
export function inferSecteursFromClasses(classes: string): Secteur[] {
  const c = String(classes ?? "");
  const u = c.toUpperCase().replace(/°/g, "");
  const out = new Set<Secteur>();
  if (/c\.?e\.?|cm[12]|\bcp\b|je\d|gs\b|ms\b|ps\b|maternelle|primaire/i.test(c)) out.add("ecole");
  if (/\d+\s*°|[3456]°|[3456]\s*[a-f]/i.test(c)) out.add("college");
  if (/\bT[BCDEF]\b|\b2[A-F]\b|\b1[A-F]\b/.test(u)) out.add("lycee");
  return [...out];
}

function personKey(nom: string, prenom: string, secteur: Secteur): string {
  return `${normHeader(nom)}|${normHeader(prenom)}|${secteur}`;
}

export function mergeEnseignantsLists(
  existing: EnseignantConfig[],
  incoming: EnseignantConfig[],
): { enseignants: EnseignantConfig[]; stats: EnseignantsMergeStats } {
  const result = [...existing];
  const touched = new Set<number>();
  let added = 0;
  let updated = 0;

  for (const inc of incoming) {
    const idx = result.findIndex(
      (e) => personKey(e.nom, e.prenom, e.secteur) === personKey(inc.nom, inc.prenom, inc.secteur),
    );
    if (idx >= 0) {
      result[idx] = {
        ...result[idx],
        ...inc,
        id: result[idx].id,
        folderName: inc.folderName || result[idx].folderName,
        email: inc.email || result[idx].email,
        emailPro: inc.emailPro || result[idx].emailPro,
      };
      touched.add(idx);
      updated++;
    } else {
      result.push(inc);
      added++;
    }
  }

  return {
    enseignants: result,
    stats: {
      total: result.length,
      added,
      updated,
      kept: existing.length - touched.size,
    },
  };
}

export function parseEnseignantsExcelBuffer(buffer: ArrayBuffer): EnseignantsImportResult {
  let workbook: XLSX.WorkBook;
  try {
    workbook = XLSX.read(buffer, { type: "array", cellDates: false });
  } catch {
    return { ok: false, error: "Fichier Excel illisible." };
  }

  const sheetName = workbook.SheetNames[0];
  if (!sheetName) return { ok: false, error: "Aucune feuille dans le fichier." };

  const rows = XLSX.utils.sheet_to_json<unknown[]>(workbook.Sheets[sheetName]!, {
    header: 1,
    defval: "",
    raw: false,
  }) as unknown[][];

  if (rows.length < 2) return { ok: false, error: "Fichier vide ou sans données." };

  const headers = (rows[0] || []).map(normHeader);
  const colNom = findCol(headers, ["nom", "nom de famille", "nom famille", "name", "nom prof"]);
  const colPrenom = findCol(headers, ["prenom", "prénom", "firstname", "first name"]);
  const colClasses = findCol(headers, [
    "liste des classes",
    "classes",
    "classe",
    "affectation",
    "liste classes",
  ]);
  const { perso: colEmailPerso, pro: colEmailPro } = resolveEmailColumns(headers);
  const colSecteur = findCol(headers, ["secteur", "cycle", "niveau", "etablissement", "établissement"]);

  if (colNom < 0 && colPrenom < 0) {
    return {
      ok: false,
      error: "Colonnes « Nom » et « Prénom » introuvables (1re ligne = en-têtes).",
    };
  }

  const enseignants: EnseignantConfig[] = [];
  const skipped: string[] = [];
  const warnings: string[] = [];

  for (let i = 1; i < rows.length; i++) {
    const row = rows[i] || [];
    let nom = cellStr(row, colNom);
    let prenom = cellStr(row, colPrenom);

    if (!nom && !prenom) {
      const full = cellStr(row, findCol(headers, ["nom complet", "professeur", "enseignant", "prof"]));
      if (full) {
        const parts = full.split(/\s+/).filter(Boolean);
        if (parts.length >= 2) {
          nom = parts[0]!;
          prenom = parts.slice(1).join(" ");
        } else {
          nom = full;
        }
      }
    }

    if (!nom || !prenom) {
      skipped.push(`Ligne ${i + 1} : nom ou prénom manquant.`);
      continue;
    }

    const classes = cellStr(row, colClasses);
    const email = normEmail(cellStr(row, colEmailPerso));
    const emailPro = normEmail(cellStr(row, colEmailPro));
    const secteurDirect = parseSecteurLabel(cellStr(row, colSecteur));
    const secteurs = secteurDirect ? [secteurDirect] : inferSecteursFromClasses(classes);

    if (secteurs.length === 0) {
      skipped.push(`Ligne ${i + 1} (${nom} ${prenom}) : cycle non détecté — ajoutez une colonne Cycle ou des classes.`);
      continue;
    }

    if (secteurs.length > 1) {
      warnings.push(
        `${nom} ${prenom} : plusieurs cycles (${secteurs.join(", ")}) — une entrée par cycle pour les droits OCR.`,
      );
    }

    for (const secteur of secteurs) {
      enseignants.push({
        id: "",
        nom,
        prenom,
        folderName: "",
        secteur,
        email,
        emailPro,
      });
    }
  }

  if (enseignants.length === 0) {
    return {
      ok: false,
      error: skipped[0] || "Aucun enseignant valide lu dans le fichier.",
    };
  }

  return { ok: true, enseignants, skipped, warnings };
}
