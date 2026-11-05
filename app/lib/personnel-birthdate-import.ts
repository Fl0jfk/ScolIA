import * as XLSX from "xlsx";
import { validatePersonnelBirthDate } from "@/app/lib/personnel-birthdate";

export const BIRTHDATE_IMPORT_MAX_BYTES = 2 * 1024 * 1024;
export const BIRTHDATE_IMPORT_MAX_ROWS = 2000;

export type BirthdateImportStatus =
  | "ok"
  | "inchange"
  | "remplace"
  | "introuvable"
  | "ambigu"
  | "date_invalide";

export type BirthdateSheetRow = {
  line: number;
  firstName: string;
  lastName: string;
  fullName: string;
  email: string;
  rawDate: unknown;
};

export type BirthdateStaff = {
  id: string;
  firstName: string;
  lastName: string;
  emails: string[];
  birthDate: string | null;
};

export type BirthdateImportRow = {
  line: number;
  firstName: string;
  lastName: string;
  status: BirthdateImportStatus;
  oldDate: string | null;
  newDate: string | null;
  message?: string;
  /** Usage serveur uniquement (ne pas renvoyer au client). */
  staffId?: string;
};

export type BirthdateSheetParse =
  | { ok: true; rows: BirthdateSheetRow[] }
  | { ok: false; error: string };

function normHeader(v: unknown): string {
  return String(v ?? "")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/** Clé de nom indépendante de l'ordre, de la casse, des accents et des tirets. */
export function nameKey(...parts: string[]): string {
  return parts
    .join(" ")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim()
    .split(" ")
    .filter(Boolean)
    .sort()
    .join(" ");
}

type Cols = { last: number; first: number; full: number; email: number; date: number };

function classifyHeaders(headers: string[]): Cols {
  const cols: Cols = { last: -1, first: -1, full: -1, email: -1, date: -1 };
  headers.forEach((h, i) => {
    if (!h) return;
    if (h.includes("mail") || h.includes("courriel")) {
      if (cols.email < 0) cols.email = i;
      return;
    }
    if (h.includes("nom") && h.includes("naissance")) return; // nom de naissance
    if (h.includes("naiss") || h.includes("ne le") || h === "ddn" || h.includes("birth")) {
      if (cols.date < 0) cols.date = i;
      return;
    }
    // « prenom » contient « nom » : on compare des mots entiers (« nom prenom », « prenom nom »…).
    const words = h.split(" ");
    if (words.includes("prenom") && words.includes("nom")) {
      if (cols.full < 0) cols.full = i;
      return;
    }
    if (["nom complet", "identite", "name", "full name", "collaborateur", "personnel"].includes(h)) {
      if (cols.full < 0) cols.full = i;
      return;
    }
    if (h.includes("prenom") || h === "first name" || h === "firstname") {
      if (cols.first < 0) cols.first = i;
      return;
    }
    if (["nom", "nom de famille", "nom usuel", "last name", "lastname", "surname"].includes(h)) {
      if (cols.last < 0) cols.last = i;
    }
  });
  return cols;
}

function hasNameAndDate(c: Cols) {
  return c.date >= 0 && (c.full >= 0 || c.last >= 0 || c.email >= 0);
}

/** Convertit une cellule (série Excel, Date, texte FR/ISO) en AAAA-MM-JJ, sinon null. */
export function parseBirthDateCell(raw: unknown): string | null {
  const pad = (n: number) => String(n).padStart(2, "0");
  if (raw instanceof Date) {
    if (Number.isNaN(raw.getTime())) return null;
    return `${raw.getFullYear()}-${pad(raw.getMonth() + 1)}-${pad(raw.getDate())}`;
  }
  if (typeof raw === "number") {
    if (!Number.isFinite(raw) || raw < 1 || raw > 80000) return null;
    const dt = new Date(Date.UTC(1899, 11, 30) + Math.floor(raw) * 86400000);
    return `${dt.getUTCFullYear()}-${pad(dt.getUTCMonth() + 1)}-${pad(dt.getUTCDate())}`;
  }
  const s = String(raw ?? "").trim();
  if (!s) return null;
  const iso = s.match(/^(\d{4})-(\d{1,2})-(\d{1,2})(?:[T\s].*)?$/);
  if (iso) return `${iso[1]}-${pad(Number(iso[2]))}-${pad(Number(iso[3]))}`;
  const fr = s.match(/^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})$/);
  if (fr) return `${fr[3]}-${pad(Number(fr[2]))}-${pad(Number(fr[1]))}`;
  return null;
}

export function parseBirthdateSheetBuffer(buf: ArrayBuffer | Uint8Array): BirthdateSheetParse {
  let wb: XLSX.WorkBook;
  try {
    wb = XLSX.read(buf, { type: "array", cellDates: false });
  } catch {
    return { ok: false, error: "Fichier illisible (formats acceptés : .xlsx, .xls, .csv)." };
  }
  const is1904 = Boolean(wb.Workbook?.WBProps?.date1904);
  const sheet = wb.Sheets[wb.SheetNames[0] ?? ""];
  if (!sheet) return { ok: false, error: "Le fichier ne contient aucune feuille." };
  const grid = XLSX.utils.sheet_to_json<unknown[]>(sheet, {
    header: 1,
    raw: true,
    defval: "",
    blankrows: false,
  });

  let headerIdx = -1;
  let cols: Cols = { last: -1, first: -1, full: -1, email: -1, date: -1 };
  for (let i = 0; i < Math.min(grid.length, 10); i++) {
    const c = classifyHeaders((grid[i] ?? []).map(normHeader));
    if (hasNameAndDate(c)) {
      headerIdx = i;
      cols = c;
      break;
    }
  }
  if (headerIdx < 0) {
    return {
      ok: false,
      error: "Colonnes introuvables : il faut au minimum Nom, Prénom (ou Email) et Date de naissance.",
    };
  }
  const dataRows = grid.slice(headerIdx + 1);
  if (dataRows.length > BIRTHDATE_IMPORT_MAX_ROWS) {
    return { ok: false, error: `Trop de lignes (maximum ${BIRTHDATE_IMPORT_MAX_ROWS}).` };
  }
  const rows: BirthdateSheetRow[] = [];
  dataRows.forEach((r, k) => {
    const cell = (i: number): unknown => (i >= 0 ? r[i] : "");
    const txt = (i: number) => String(cell(i) ?? "").trim();
    const row: BirthdateSheetRow = {
      line: headerIdx + 2 + k,
      lastName: txt(cols.last),
      firstName: txt(cols.first),
      fullName: txt(cols.full),
      email: txt(cols.email).toLowerCase(),
      // Classeurs au calendrier 1904 (anciens Excel Mac) : ramener la série en base 1900.
      rawDate: ((v) => (is1904 && typeof v === "number" ? v + 1462 : v))(cell(cols.date)),
    };
    if (
      !row.lastName &&
      !row.firstName &&
      !row.fullName &&
      !row.email &&
      String(row.rawDate ?? "").trim() === ""
    ) {
      return;
    }
    rows.push(row);
  });
  if (rows.length === 0) return { ok: false, error: "Aucune ligne de données." };
  return { ok: true, rows };
}

/** Rapproche les lignes du fichier avec le personnel actif. Fonction pure. */
export function matchBirthdateRows(rows: BirthdateSheetRow[], staff: BirthdateStaff[]): BirthdateImportRow[] {
  const byEmail = new Map<string, BirthdateStaff[]>();
  const byName = new Map<string, BirthdateStaff[]>();
  for (const s of staff) {
    for (const e of new Set(s.emails.map((x) => x.trim().toLowerCase()).filter(Boolean))) {
      byEmail.set(e, [...(byEmail.get(e) ?? []), s]);
    }
    const k = nameKey(s.firstName, s.lastName);
    if (k) byName.set(k, [...(byName.get(k) ?? []), s]);
  }

  const out: BirthdateImportRow[] = [];
  const claimed = new Set<string>();

  for (const r of rows) {
    const base = {
      line: r.line,
      firstName: r.firstName || (r.fullName && !r.lastName ? r.fullName : ""),
      lastName: r.lastName,
    };
    const iso = parseBirthDateCell(r.rawDate);
    const check = iso ? validatePersonnelBirthDate(iso) : null;
    if (!iso || !check || !check.ok) {
      out.push({
        ...base,
        status: "date_invalide",
        oldDate: null,
        newDate: null,
        message: !iso
          ? String(r.rawDate ?? "").trim()
            ? "Format de date non reconnu."
            : "Date manquante."
          : check && !check.ok
            ? check.error
            : undefined,
      });
      continue;
    }

    let candidates: BirthdateStaff[] = [];
    if (r.email) candidates = byEmail.get(r.email) ?? [];
    if (candidates.length === 0) {
      const k = r.fullName ? nameKey(r.fullName) : nameKey(r.firstName, r.lastName);
      if (k) candidates = byName.get(k) ?? [];
    }
    if (candidates.length === 0) {
      out.push({ ...base, status: "introuvable", oldDate: null, newDate: iso });
      continue;
    }
    if (candidates.length > 1) {
      out.push({
        ...base,
        status: "ambigu",
        oldDate: null,
        newDate: iso,
        message: `${candidates.length} personnes correspondent.`,
      });
      continue;
    }
    const s = candidates[0]!;
    if (claimed.has(s.id)) {
      out.push({
        ...base,
        status: "ambigu",
        oldDate: s.birthDate,
        newDate: iso,
        message: "Personne déjà présente plus haut dans le fichier.",
      });
      continue;
    }
    claimed.add(s.id);
    const display = {
      ...base,
      firstName: s.firstName,
      lastName: s.lastName,
      oldDate: s.birthDate,
      newDate: iso,
      staffId: s.id,
    };
    if (!s.birthDate) out.push({ ...display, status: "ok" });
    else if (s.birthDate === iso) out.push({ ...display, status: "inchange" });
    else out.push({ ...display, status: "remplace" });
  }
  return out;
}

export function summarizeBirthdateRows(rows: BirthdateImportRow[]): Record<BirthdateImportStatus, number> {
  const c: Record<BirthdateImportStatus, number> = {
    ok: 0,
    inchange: 0,
    remplace: 0,
    introuvable: 0,
    ambigu: 0,
    date_invalide: 0,
  };
  for (const r of rows) c[r.status]++;
  return c;
}
