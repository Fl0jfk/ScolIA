import * as XLSX from "xlsx";
import type { ClassLevel } from "@/app/lib/class-allocation-types";
import { CLASS_LEVELS } from "@/app/lib/class-allocation-types";

type ClassImportRow = {
  level: ClassLevel;
  type: "actuelle" | "cible";
  className: string;
};

type ClassImportResult =
  | { ok: true; rows: ClassImportRow[] }
  | { ok: false; error: string };

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

function parseLevel(raw: string): ClassLevel | null {
  const v = normHeader(raw);
  if (!v) return null;
  if (v === "ecole" || v.includes("elementaire") || v.includes("primaire")) return "ecole";
  if (v === "college" || v.includes("colleg")) return "college";
  if (v === "lycee" || v.includes("lyce")) return "lycee";
  return null;
}

function parseType(raw: string): "actuelle" | "cible" | null {
  const v = normHeader(raw);
  if (!v) return null;
  if (
    v === "actuelle" ||
    v === "actuel" ||
    v === "source" ||
    v === "origine" ||
    v === "depart" ||
    v.includes("actuelle")
  ) {
    return "actuelle";
  }
  if (
    v === "cible" ||
    v === "cibles" ||
    v === "destination" ||
    v === "nouvelle" ||
    v === "nouvelles" ||
    v === "arrivee" ||
    v.includes("cible")
  ) {
    return "cible";
  }
  return null;
}

function cellStr(row: unknown[], idx: number): string {
  if (idx < 0) return "";
  return String(row[idx] ?? "").trim();
}

export function parseClassAllocationClassesExcelBuffer(buffer: ArrayBuffer): ClassImportResult {
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
  const colLevel = findCol(headers, ["niveau", "cycle", "secteur", "etablissement"]);
  const colType = findCol(headers, ["type", "role", "categorie", "catégorie"]);
  const colClass = findCol(headers, ["classe", "division", "groupe", "class"]);

  if (colClass < 0) {
    return { ok: false, error: "Colonne « Classe » introuvable." };
  }

  const rowsOut: ClassImportRow[] = [];
  const errors: string[] = [];

  for (let i = 1; i < rows.length; i++) {
    const row = rows[i];
    if (!Array.isArray(row)) continue;
    const className = cellStr(row, colClass);
    if (!className) continue;

    const level = colLevel >= 0 ? parseLevel(cellStr(row, colLevel)) : null;
    const type = colType >= 0 ? parseType(cellStr(row, colType)) : null;

    if (!level) {
      errors.push(`Ligne ${i + 1} : niveau manquant ou invalide (« École », « Collège » ou « Lycée »).`);
      continue;
    }
    if (!type) {
      errors.push(`Ligne ${i + 1} : type manquant ou invalide (« actuelle » ou « cible »).`);
      continue;
    }

    rowsOut.push({ level, type, className });
  }

  if (!rowsOut.length) {
    const hint =
      colLevel < 0 || colType < 0
        ? "Colonnes attendues : Niveau, Type (actuelle ou cible), Classe."
        : "Aucune ligne valide trouvée.";
    return { ok: false, error: errors[0] || hint };
  }

  if (errors.length > 3) {
    return { ok: false, error: `${errors.slice(0, 3).join(" ")} (${errors.length} erreurs au total)` };
  }
  if (errors.length) {
    return { ok: false, error: errors.join(" ") };
  }

  return { ok: true, rows: rowsOut };
}

export function mergeClassImportRows(
  levels: { level: ClassLevel; sourceClassPrefixes: string[]; targetClasses: string[] }[],
  rows: ClassImportRow[],
  mode: "replace" | "merge" = "replace",
): { level: ClassLevel; sourceClassPrefixes: string[]; targetClasses: string[] }[] {
  const map = new Map<ClassLevel, { source: Set<string>; target: Set<string> }>();
  for (const lvl of CLASS_LEVELS) {
    const existing = levels.find((l) => l.level === lvl);
    map.set(lvl, {
      source: new Set(mode === "merge" ? existing?.sourceClassPrefixes || [] : []),
      target: new Set(mode === "merge" ? existing?.targetClasses || [] : []),
    });
  }

  for (const row of rows) {
    const bucket = map.get(row.level);
    if (!bucket) continue;
    if (row.type === "actuelle") bucket.source.add(row.className);
    else bucket.target.add(row.className);
  }

  return CLASS_LEVELS.map((level) => {
    const bucket = map.get(level)!;
    return {
      level,
      sourceClassPrefixes: Array.from(bucket.source),
      targetClasses: Array.from(bucket.target),
    };
  }).filter((l) => l.sourceClassPrefixes.length > 0 || l.targetClasses.length > 0);
}
