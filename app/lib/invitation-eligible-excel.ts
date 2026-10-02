/**
 * Import Excel — liste élèves autorisés (invitations cérémonies).
 * Colonnes flexibles : Prénom, Nom, Date de naissance, Diplôme (bac/brevet).
 */
import * as XLSX from "xlsx";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import {
  isInvitationDiploma,
  parseInvitationBirthDate,
  type InvitationDiploma,
} from "@/app/lib/invitation-types";

export type InvitationEligibleExcelRow = {
  eleveFirstName: string;
  eleveLastName: string;
  birthDate: string | null;
  diploma: InvitationDiploma | null;
};

function foldHeader(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/['"`]/g, "")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/**
 * Correspondance stricte : « prenom » ne doit jamais matcher l’alias « nom »
 * (sous-chaîne). Exact d’abord, puis alias multi-mots via includes.
 */
function headerMatchesAlias(header: string, alias: string): boolean {
  if (!header || !alias) return false;
  if (header === alias) return true;
  if (
    !alias.includes(" ") &&
    ["nom", "prenom", "name", "diploma", "diplome", "bac", "brevet"].includes(alias)
  ) {
    return false;
  }
  if (alias.includes(" ")) return header.includes(alias);
  return (
    header.startsWith(`${alias} `) ||
    header.endsWith(` ${alias}`) ||
    header.includes(` ${alias} `)
  );
}

function findCol(headers: string[], aliases: string[]): number {
  for (let i = 0; i < headers.length; i++) {
    const h = headers[i]!;
    if (aliases.some((a) => headerMatchesAlias(h, a))) return i;
  }
  return -1;
}

function cellToBirthDate(raw: unknown): string | null {
  if (raw == null || raw === "") return null;
  const fromEleves = normalizeEleveDateNaissance(raw);
  if (fromEleves) {
    return parseInvitationBirthDate(fromEleves) || fromEleves;
  }
  return parseInvitationBirthDate(String(raw));
}

function cellToDiploma(raw: unknown): InvitationDiploma | null {
  const s = String(raw ?? "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
  if (!s) return null;
  if (s === "bac" || s.includes("baccalaureat") || s.includes("baccalaure")) return "bac";
  if (s === "brevet" || s.includes("dnb") || s.includes("diplome national")) return "brevet";
  if (isInvitationDiploma(s)) return s;
  return null;
}

export type ParseEligibleExcelResult =
  | { ok: true; rows: InvitationEligibleExcelRow[] }
  | { ok: false; error: string };

export function parseInvitationEligibleExcelBuffer(
  buf: ArrayBuffer | Buffer,
): ParseEligibleExcelResult {
  const wb = XLSX.read(buf, { type: "buffer", cellDates: true });
  const sheetName = wb.SheetNames[0];
  if (!sheetName) return { ok: false, error: "Fichier Excel vide." };
  const sheet = wb.Sheets[sheetName];
  if (!sheet) return { ok: false, error: "Feuille Excel introuvable." };

  const matrix = XLSX.utils.sheet_to_json<(string | number | Date | null)[]>(sheet, {
    header: 1,
    defval: "",
    raw: true,
  }) as (string | number | Date | null)[][];

  if (matrix.length < 2) {
    return {
      ok: false,
      error: "Le fichier doit contenir une ligne d’en-tête et au moins une ligne élève.",
    };
  }

  // Trouver la ligne d’en-tête (premières lignes)
  let headerRowIdx = 0;
  for (let r = 0; r < Math.min(8, matrix.length); r++) {
    const folded = (matrix[r] || []).map((c) => foldHeader(String(c ?? "")));
    const hasPrenom = folded.some(
      (h) => h === "prenom" || h.includes("prenom") || h === "firstname",
    );
    const hasNom = folded.some(
      (h) =>
        h === "nom" ||
        h.includes("nom de famille") ||
        h === "nom eleve" ||
        h === "lastname" ||
        h === "name",
    );
    if (hasPrenom && hasNom) {
      headerRowIdx = r;
      break;
    }
  }

  const headers = (matrix[headerRowIdx] || []).map((c) => foldHeader(String(c ?? "")));
  const iPrenom = findCol(headers, [
    "prenom",
    "prenom eleve",
    "prenom de l eleve",
    "firstname",
    "first name",
  ]);
  const iNom = findCol(headers, [
    "nom de famille",
    "nom eleve",
    "nom de l eleve",
    "nom",
    "lastname",
    "last name",
    "name",
  ]);
  const iBirth = findCol(headers, [
    "date de naissance",
    "date naissance",
    "naissance",
    "ddn",
    "ne le",
    "nee le",
    "birth",
    "birthday",
    "date of birth",
    "dob",
  ]);
  const iDiploma = findCol(headers, [
    "diplome",
    "diploma",
    "type diplome",
    "bac",
    "brevet",
    "examen",
  ]);

  if (iPrenom < 0 || iNom < 0) {
    return {
      ok: false,
      error:
        "Colonnes « Prénom » et « Nom » introuvables. Ajoutez une ligne d’en-tête avec au minimum Prénom et Nom (idéalement Date de naissance).",
    };
  }

  const rows: InvitationEligibleExcelRow[] = [];
  for (const row of matrix.slice(headerRowIdx + 1)) {
    const eleveFirstName = String(row[iPrenom] ?? "").trim().slice(0, 80);
    const eleveLastName = String(row[iNom] ?? "").trim().slice(0, 80);
    if (!eleveFirstName || !eleveLastName) continue;
    const birthDate = iBirth >= 0 ? cellToBirthDate(row[iBirth]) : null;
    const diploma = iDiploma >= 0 ? cellToDiploma(row[iDiploma]) : null;
    rows.push({ eleveFirstName, eleveLastName, birthDate, diploma });
  }

  if (rows.length === 0) {
    return { ok: false, error: "Aucune ligne élève exploitable dans le fichier." };
  }
  return { ok: true, rows };
}
