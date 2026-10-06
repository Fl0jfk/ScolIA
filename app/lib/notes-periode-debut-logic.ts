/** 1er septembre de l’année scolaire (libellé AAAA-AAAA). */
export function defaultSchoolYearStartIsoFromLabel(label: string): string {
  const m = /^(\d{4})-(\d{4})$/.exec(label.trim());
  if (m) return `${m[1]}-09-01`;
  const y = new Date().getFullYear();
  return `${y}-09-01`;
}

export function formatSqlDateToIso(raw: unknown): string | null {
  if (raw == null) return null;
  const s = String(raw).trim();
  if (!s) return null;
  return s.slice(0, 10);
}
