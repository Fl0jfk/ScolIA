import * as XLSX from "xlsx";
import type { ClassAllocationTeacherAssignment } from "@/app/lib/class-allocation-teachers";

type TeacherRosterImportResult =
  | { ok: true; assignments: ClassAllocationTeacherAssignment[] }
  | { ok: false; error: string };

type ClerkLookup = { clerkUserId: string; email: string; displayName: string };

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

function cellStr(row: unknown[], idx: number): string {
  if (idx < 0) return "";
  return String(row[idx] ?? "").trim();
}

function resolveTeacher(
  email: string,
  name: string,
  users: ClerkLookup[],
): ClerkLookup | null {
  const mail = email.trim().toLowerCase();
  if (mail) {
    const byEmail = users.find((u) => u.email.toLowerCase() === mail);
    if (byEmail) return byEmail;
  }
  const n = name.trim().toLowerCase();
  if (!n) return null;
  return (
    users.find((u) => u.displayName.toLowerCase() === n) ||
    users.find((u) => u.displayName.toLowerCase().includes(n) || n.includes(u.displayName.toLowerCase())) ||
    null
  );
}

export function parseTeacherRosterExcelBuffer(
  buffer: ArrayBuffer,
  users: ClerkLookup[],
): TeacherRosterImportResult {
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
  const colClass = findCol(headers, ["classe", "division", "groupe", "class"]);
  const colEmail = findCol(headers, ["email", "e-mail", "mail", "courriel", "email prof", "mail professeur"]);
  const colName = findCol(headers, ["professeur", "prof", "enseignant", "referent", "référent", "nom prof"]);
  if (colClass < 0) return { ok: false, error: "Colonne « classe » introuvable." };
  if (colEmail < 0 && colName < 0) {
    return { ok: false, error: "Colonne « email » ou « professeur » requise." };
  }

  const assignments: ClassAllocationTeacherAssignment[] = [];
  const errors: string[] = [];

  for (let i = 1; i < rows.length; i++) {
    const row = rows[i] || [];
    const className = cellStr(row, colClass);
    if (!className) continue;
    const teacher = resolveTeacher(cellStr(row, colEmail), cellStr(row, colName), users);
    if (!teacher) {
      errors.push(`Ligne ${i + 1} (${className}) : professeur non reconnu.`);
      continue;
    }
    assignments.push({
      className,
      clerkUserId: teacher.clerkUserId,
      name: teacher.displayName,
      email: teacher.email,
    });
  }

  if (!assignments.length) {
    return {
      ok: false,
      error: errors[0] || "Aucune affectation valide lue.",
    };
  }

  return { ok: true, assignments };
}
