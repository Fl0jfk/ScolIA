/**
 * Parse / regroupement Excel mails parents (Charlemagne).
 * Plusieurs lignes même nom/prénom = plusieurs foyers ; mêmes e-mails = un seul foyer.
 */

import * as XLSX from "xlsx";
import { identityKey } from "@/app/lib/eleve-photos-match";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function normalizeParentEmail(email: string): string {
  return email.trim().toLowerCase();
}

function isValidParentEmail(email: string): boolean {
  return EMAIL_RE.test(normalizeParentEmail(email));
}

export type ParentEmailExcelLine = {
  nom: string;
  prenom: string;
  emails: string[];
};

export type ParentEmailStudentGroup = {
  nom: string;
  prenom: string;
  key: string;
  /** Foyers dédupliqués (ensemble d’e-mails unique par ligne). */
  foyers: string[][];
  /** Union de tous les e-mails. */
  allEmails: string[];
};

export function uniqParentEmails(raw: string[]): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const r of raw) {
    const e = normalizeParentEmail(r);
    if (!isValidParentEmail(e) || seen.has(e)) continue;
    seen.add(e);
    out.push(e);
  }
  return out;
}

function emailSetKey(emails: string[]): string {
  return [...emails].sort().join("|");
}

/** Parse le classeur export (ligne d’en-tête « Nom / Prénom / Email… »). */
export function parseParentEmailsExcelBuffer(buf: Buffer): ParentEmailExcelLine[] {
  const wb = XLSX.read(buf, { type: "buffer" });
  const sheet = wb.Sheets[wb.SheetNames[0]!];
  if (!sheet) return [];
  const raw = XLSX.utils.sheet_to_json<(string | number | null)[]>(sheet, {
    header: 1,
    defval: "",
    raw: false,
  }) as string[][];

  const headerIdx = raw.findIndex(
    (r) => String(r[0] || "").trim().toLowerCase() === "nom",
  );
  if (headerIdx < 0) {
    throw new Error('En-tête « Nom » introuvable dans le fichier Excel.');
  }

  const lines: ParentEmailExcelLine[] = [];
  for (const row of raw.slice(headerIdx + 1)) {
    const nom = String(row[0] || "").trim();
    const prenom = String(row[1] || "").trim();
    if (!nom && !prenom) continue;
    const emails = uniqParentEmails([
      String(row[2] || ""),
      String(row[3] || ""),
      String(row[4] || ""),
    ]);
    lines.push({ nom, prenom, emails });
  }
  return lines;
}

/** Regroupe par élève ; lignes au même jeu d’e-mails → un seul foyer. */
export function groupParentEmailLines(
  lines: ParentEmailExcelLine[],
): ParentEmailStudentGroup[] {
  const byKey = new Map<
    string,
    { nom: string; prenom: string; foyerKeys: Map<string, string[]> }
  >();

  for (const line of lines) {
    const key = identityKey(line.nom, line.prenom);
    if (!key || key === "§") continue;
    let g = byKey.get(key);
    if (!g) {
      g = { nom: line.nom, prenom: line.prenom, foyerKeys: new Map() };
      byKey.set(key, g);
    }
    if (!line.emails.length) continue;
    const fk = emailSetKey(line.emails);
    if (!g.foyerKeys.has(fk)) g.foyerKeys.set(fk, line.emails);
  }

  return [...byKey.entries()].map(([key, g]) => {
    const foyers = [...g.foyerKeys.values()];
    const all = uniqParentEmails(foyers.flat());
    return {
      nom: g.nom,
      prenom: g.prenom,
      key,
      foyers,
      allEmails: all,
    };
  });
}
