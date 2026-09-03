/**
 * Helpers date purs pour absences Postgres / Drizzle.
 * Les drivers peuvent renvoyer `date`/`timestamptz` en string OU en Date :
 * appeler `.toISOString()` à l’aveugle plante list/create/validate.
 */

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

/** Clé calendaire `YYYY-MM-DD` (sans décalage fuseau pour minuit UTC). */
export function toAbsenceDateOnly(raw: string | Date | null | undefined): string {
  if (raw instanceof Date && !Number.isNaN(raw.getTime())) {
    if (
      raw.getUTCHours() === 0 &&
      raw.getUTCMinutes() === 0 &&
      raw.getUTCSeconds() === 0 &&
      raw.getUTCMilliseconds() === 0
    ) {
      return `${raw.getUTCFullYear()}-${pad2(raw.getUTCMonth() + 1)}-${pad2(raw.getUTCDate())}`;
    }
    return raw.toISOString().slice(0, 10);
  }
  const s = String(raw ?? "").trim();
  if (/^\d{4}-\d{2}-\d{2}/.test(s)) return s.slice(0, 10);
  const d = new Date(s);
  if (Number.isNaN(d.getTime())) return new Date().toISOString().slice(0, 10);
  return toAbsenceDateOnly(d);
}

/** ISO-8601 instant — accepte Date ou string déjà ISO. */
export function toAbsenceIsoTimestamp(
  raw: string | Date | null | undefined,
  fallback?: string,
): string {
  if (raw instanceof Date && !Number.isNaN(raw.getTime())) {
    return raw.toISOString();
  }
  if (typeof raw === "string") {
    const s = raw.trim();
    if (!s) {
      if (fallback) return fallback;
      return new Date().toISOString();
    }
    // Déjà ISO
    if (/^\d{4}-\d{2}-\d{2}T/.test(s)) {
      const d = new Date(s);
      return Number.isNaN(d.getTime()) ? s : d.toISOString();
    }
    // Date seule → minuit UTC
    if (/^\d{4}-\d{2}-\d{2}$/.test(s)) {
      return `${s}T00:00:00.000Z`;
    }
    const d = new Date(s);
    if (!Number.isNaN(d.getTime())) return d.toISOString();
    if (fallback) return fallback;
    return new Date().toISOString();
  }
  if (fallback) return fallback;
  return new Date().toISOString();
}

export function toAbsenceIsoTimestampOrNull(
  raw: string | Date | null | undefined,
): string | null {
  if (raw == null) return null;
  if (typeof raw === "string" && !raw.trim()) return null;
  return toAbsenceIsoTimestamp(raw);
}
