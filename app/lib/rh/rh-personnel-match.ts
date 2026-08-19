import type { PersonnelExtracted } from "@/app/lib/personnel-match";
import { normalizeNir } from "@/app/lib/personnel-match";
import { collectOcrEmails, matchExtractedEmailAgainstList, normalizeOcrEmail } from "@/app/lib/ocr-email-match";
import type { RhPersonnelIndexEntry } from "@/app/lib/rh/types";

function personnelEmails(entry: RhPersonnelIndexEntry): string[] {
  return collectOcrEmails(entry.email, entry.emailPerso, entry.emailPro);
}

function normalize(str: string) {
  return String(str || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[-\s_]+/g, " ")
    .trim();
}

function nameScore(nom: string, prenom: string, entry: RhPersonnelIndexEntry): number {
  const display = normalize(entry.displayName);
  const folder = normalize(entry.folderName.replace(/_/g, " "));
  const n = normalize(nom);
  const p = normalize(prenom);
  let score = 0;
  if (n && (display.includes(n) || folder.includes(n))) score += 3;
  if (p && (display.includes(p) || folder.includes(p))) score += 3;
  if (n && p && display.includes(n) && display.includes(p)) score += 4;
  return score;
}

export type RhPersonnelMatchResult = {
  entry: RhPersonnelIndexEntry | null;
  score: number;
  candidates: RhPersonnelIndexEntry[];
  matchedBy?: "email" | "nir" | "name";
};

/** Match local contre l'index OneDrive RH (sans charger tous les meta-rh). */
export function matchRhPersonnelFromIndex(
  extracted: PersonnelExtracted,
  entries: RhPersonnelIndexEntry[],
  nirByPersonnelId?: Map<string, string>,
): RhPersonnelMatchResult {
  const active = entries.filter((e) => e.active !== false);

  const extractedHit = matchExtractedEmailAgainstList(extracted.email, active, personnelEmails);
  if (extractedHit) {
    return { entry: extractedHit, score: 100, candidates: [extractedHit], matchedBy: "email" };
  }

  const email = normalizeOcrEmail(extracted.email);
  if (email) {
    const hits = active.filter((e) => personnelEmails(e).includes(email));
    if (hits.length === 1) return { entry: hits[0], score: 100, candidates: hits, matchedBy: "email" };
    if (hits.length > 1) {
      return { entry: null, score: 100, candidates: hits.slice(0, 5), matchedBy: "email" };
    }
  }

  const nir = normalizeNir(extracted.numero_securite_sociale);
  if (nir.length >= 13 && nirByPersonnelId?.size) {
    const hits = active.filter((e) => nirByPersonnelId.get(e.id) === nir);
    if (hits.length === 1) return { entry: hits[0], score: 100, candidates: hits, matchedBy: "nir" };
    if (hits.length > 1) return { entry: null, score: 100, candidates: hits.slice(0, 5), matchedBy: "nir" };
  }

  const nom = extracted.nom || "";
  const prenom = extracted.prenom || "";
  if (!nom && !prenom) return { entry: null, score: 0, candidates: [] };

  const scored = active
    .map((e) => ({ e, s: nameScore(nom, prenom, e) }))
    .filter((x) => x.s >= 4)
    .sort((a, b) => b.s - a.s);

  if (scored.length === 0) return { entry: null, score: 0, candidates: [] };
  if (scored.length === 1 || scored[0].s > scored[1].s + 1) {
    return { entry: scored[0].e, score: scored[0].s, candidates: scored.slice(0, 3).map((x) => x.e), matchedBy: "name" };
  }
  return {
    entry: null,
    score: scored[0].s,
    candidates: scored.slice(0, 5).map((x) => x.e),
    matchedBy: "name",
  };
}

export function resolveRhIndexEntryForUser(
  entries: RhPersonnelIndexEntry[],
  clerkUserId?: string | null,
  email?: string | null,
): RhPersonnelIndexEntry | null {
  const normalizedEmail = email?.trim().toLowerCase();
  if (clerkUserId) {
    const byClerk = entries.find((e) => e.clerkUserId === clerkUserId && e.active !== false);
    if (byClerk) return byClerk;
  }
  if (normalizedEmail) {
    return (
      entries.find(
        (e) => personnelEmails(e).includes(normalizedEmail) && e.active !== false,
      ) ?? null
    );
  }
  return null;
}
