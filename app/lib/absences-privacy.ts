import { resolveAbsenceScope, type AbsenceRecord } from "@/app/lib/absences-types";

const SENSITIVE_PATTERNS = [
  /enfant\s*malade/i,
  /arret\s*maladie/i,
  /arrêt\s*maladie/i,
  /\bmaladie\b/i,
  /certificat\s*m[eé]dical/i,
  /hospitalisation/i,
  /accident\s*(du\s*travail|de\s*travail)?/i,
  /\bmaternit[eé]\b/i,
  /\bpaternit[eé]\b/i,
  /grossesse/i,
  /burn[\s-]?out/i,
  /arrêt\s*de\s*travail/i,
  /visite\s*m[eé]dicale/i,
];

const NON_SENSITIVE_PATTERNS = [
  /convocation/i,
  /\bbac\b/i,
  /examen/i,
  /formation/i,
  /jury/i,
  /réunion/i,
  /reunion/i,
];

function normalizePrivacyText(...parts: Array<string | undefined | null>) {
  return parts
    .filter(Boolean)
    .join(" ")
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
}

/** Détecte arrêt maladie et autres motifs/documents sensibles (pas convocation bac, etc.). */
export function isSensitiveAbsenceContent(
  reason: string | undefined,
  details?: string | null,
  fileName?: string | null,
) {
  const text = normalizePrivacyText(reason, details, fileName);
  if (!text.trim()) return false;

  const clearlySensitive = SENSITIVE_PATTERNS.some((pattern) => pattern.test(text));
  if (!clearlySensitive) return false;

  const onlyAdministrative =
    NON_SENSITIVE_PATTERNS.some((pattern) => pattern.test(text)) &&
    !/maladie|arret\s*maladie|arrêt\s*maladie|certificat\s*medical|certificat\s*médical|hospitalisation/i.test(
      text,
    );

  return !onlyAdministrative;
}

/** Libellé affiché au calendrier / dashboard (sans motif médical ni motif OGEC). */
export function getPublicAbsenceReason(record: AbsenceRecord): string {
  if (record.privacyReasonRedacted) return "Absence";
  if (resolveAbsenceScope(record) === "ogec") return "Absence";
  if (isSensitiveAbsenceContent(record.data.reason, record.data.details, record.justification?.fileName)) {
    return "Absence";
  }
  return record.data.reason?.trim() || "Absence";
}
