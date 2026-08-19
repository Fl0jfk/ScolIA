import * as XLSX from "xlsx";
import type { PersonnelCategory } from "@/app/lib/personnel-types";

export type PersonnelImportRow = {
  firstName: string;
  lastName: string;
  emailPerso?: string;
  emailPro?: string;
  category: PersonnelCategory;
  jobTitle?: string;
};

export type PersonnelImportResult =
  | { ok: true; rows: PersonnelImportRow[]; skipped: string[] }
  | { ok: false; error: string };

const CATEGORY_ALIASES: Record<PersonnelCategory, string[]> = {
  administratif: ["administratif", "admin", "secretariat", "secrétariat", "accueil"],
  comptabilite: ["comptabilite", "comptabilité", "compta", "finance"],
  cpe: ["cpe", "vie scolaire"],
  education: ["education", "éducation", "surveillance", "aed", "assistant"],
  maintenance: ["maintenance", "technique", "intendance"],
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

function cellStr(row: unknown[], idx: number): string {
  if (idx < 0) return "";
  return String(row[idx] ?? "").trim();
}

function normEmail(raw: string): string | undefined {
  const e = raw.trim().toLowerCase();
  return e && e.includes("@") ? e : undefined;
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
  return { perso: perso >= 0 ? perso : generic, pro };
}

function parseCategoryFromFonction(raw: string): PersonnelCategory {
  const s = normHeader(raw);
  if (!s) return "administratif";
  for (const [cat, aliases] of Object.entries(CATEGORY_ALIASES) as [PersonnelCategory, string[]][]) {
    if (aliases.some((a) => s === a || s.includes(a))) return cat;
  }
  if (/compta|comptable|finance|paie|treasury/.test(s)) return "comptabilite";
  if (/cpe|vie scolaire|assistante d/.test(s)) return "cpe";
  if (/surveill|aed|agent|veilleur|education/.test(s)) return "education";
  if (/maintenance|technicien|intendant|agent d entretien/.test(s)) return "maintenance";
  if (/secretaire|secretariat|accueil|administratif|direction adjointe/.test(s)) return "administratif";
  return "administratif";
}

function splitFullName(full: string): { lastName: string; firstName: string } | null {
  const parts = full.trim().split(/\s+/).filter(Boolean);
  if (parts.length < 2) return null;
  const firstIsCaps = parts[0] === parts[0].toUpperCase() && /[A-ZÀ-Ÿ]/.test(parts[0]);
  if (firstIsCaps) {
    return { lastName: parts[0]!, firstName: parts.slice(1).join(" ") };
  }
  return { lastName: parts[parts.length - 1]!, firstName: parts.slice(0, -1).join(" ") };
}

export function parsePersonnelExcelBuffer(buffer: ArrayBuffer): PersonnelImportResult {
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
  const colNom = findCol(headers, ["nom", "nom de famille", "nom famille", "lastname", "last name"]);
  const colPrenom = findCol(headers, ["prenom", "prénom", "firstname", "first name"]);
  const { perso: colEmailPerso, pro: colEmailPro } = resolveEmailColumns(headers);
  const colFonction = findCol(headers, ["fonction", "fonctions", "intitule", "intitulé", "poste"]);
  const colCategory = findCol(headers, ["categorie", "catégorie", "service", "pole", "pôle"]);
  const colFull = findCol(headers, ["nom complet", "collaborateur", "salarié", "salarie", "personnel"]);

  if (colEmailPerso < 0 && colEmailPro < 0) {
    return {
      ok: false,
      error: "Au moins une colonne email requise : « Email personnel » et/ou « Email professionnel ».",
    };
  }

  const parsed: PersonnelImportRow[] = [];
  const skipped: string[] = [];

  for (let i = 1; i < rows.length; i++) {
    const row = rows[i] || [];
    let lastName = cellStr(row, colNom);
    let firstName = cellStr(row, colPrenom);
    const emailPerso = normEmail(cellStr(row, colEmailPerso));
    const emailPro = normEmail(cellStr(row, colEmailPro));

    if ((!lastName || !firstName) && colFull >= 0) {
      const split = splitFullName(cellStr(row, colFull));
      if (split) {
        lastName = split.lastName;
        firstName = split.firstName;
      }
    }

    if (!emailPerso && !emailPro) {
      skipped.push(`Ligne ${i + 1} : aucun email (perso ou pro).`);
      continue;
    }
    if (!lastName || !firstName) {
      skipped.push(`Ligne ${i + 1} : nom ou prénom manquant.`);
      continue;
    }

    const fonction = cellStr(row, colFonction);
    const categoryRaw = cellStr(row, colCategory) || fonction;

    parsed.push({
      lastName,
      firstName,
      emailPerso,
      emailPro,
      category: parseCategoryFromFonction(categoryRaw),
      jobTitle: fonction || undefined,
    });
  }

  if (parsed.length === 0) {
    return { ok: false, error: skipped[0] || "Aucune ligne personnel valide." };
  }

  return { ok: true, rows: parsed, skipped };
}
