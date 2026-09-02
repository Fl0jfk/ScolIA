/** Durées d’accès temporaire aux documents élèves (ex. PAP). */
export const DOCUMENT_ACCESS_DURATION_OPTIONS = [
  { days: 1, label: "1 jour" },
  { days: 7, label: "1 semaine" },
  { days: 30, label: "1 mois" },
  { days: 365, label: "1 année" },
] as const;

export type DocumentAccessDurationDays =
  (typeof DOCUMENT_ACCESS_DURATION_OPTIONS)[number]["days"];

const ALLOWED = new Set<number>(
  DOCUMENT_ACCESS_DURATION_OPTIONS.map((o) => o.days),
);

/** Normalise une durée demandée / choisie par la direction. */
export function normalizeDocumentAccessDurationDays(
  raw: unknown,
  fallback: DocumentAccessDurationDays = 7,
): DocumentAccessDurationDays {
  const n = Number(raw);
  if (ALLOWED.has(n)) return n as DocumentAccessDurationDays;
  return fallback;
}

export function documentAccessDurationLabel(days: number): string {
  const hit = DOCUMENT_ACCESS_DURATION_OPTIONS.find((o) => o.days === days);
  return hit?.label ?? `${days} jour${days > 1 ? "s" : ""}`;
}
