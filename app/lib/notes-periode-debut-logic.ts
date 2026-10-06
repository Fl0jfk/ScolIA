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

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export function isUuidV4Like(id: string): boolean {
  return UUID_RE.test(id.trim());
}

/** Début d’année depuis le calendrier : ignore les périodes de vacances. */
export function pickCalendrierAnneeDebutIso(
  entries: Array<{ dateDebut: unknown; type: string | null }>,
): string | null {
  const sorted = [...entries].sort((a, b) =>
    String(a.dateDebut ?? "").localeCompare(String(b.dateDebut ?? "")),
  );
  for (const row of sorted) {
    const kind = String(row.type ?? "").trim().toLowerCase();
    if (kind === "vacances" || kind === "vacance") continue;
    const iso = formatSqlDateToIso(row.dateDebut);
    if (iso) return iso;
  }
  return null;
}
