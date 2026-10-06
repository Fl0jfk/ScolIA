/**
 * Parse une date de séjour stockée en texte (ISO, FR jj/mm/aaaa, jj.mm.aaaa).
 * Retourne un timestamp début de journée locale, ou null si invalide.
 */
export function parseTravelDayStartMs(raw: string | null | undefined): number | null {
  const s = String(raw ?? "").trim();
  if (!s) return null;

  const iso = /^(\d{4})-(\d{2})-(\d{2})/.exec(s);
  if (iso) {
    const y = Number(iso[1]);
    const mo = Number(iso[2]) - 1;
    const d = Number(iso[3]);
    const dt = new Date(y, mo, d);
    if (dt.getFullYear() === y && dt.getMonth() === mo && dt.getDate() === d) {
      dt.setHours(0, 0, 0, 0);
      return dt.getTime();
    }
  }

  const fr = /^(\d{1,2})[./-](\d{1,2})[./-](\d{4})/.exec(s);
  if (fr) {
    const d = Number(fr[1]);
    const mo = Number(fr[2]) - 1;
    const y = Number(fr[3]);
    const dt = new Date(y, mo, d);
    if (dt.getFullYear() === y && dt.getMonth() === mo && dt.getDate() === d) {
      dt.setHours(0, 0, 0, 0);
      return dt.getTime();
    }
  }

  const fallback = new Date(s);
  if (Number.isNaN(fallback.getTime())) return null;
  fallback.setHours(0, 0, 0, 0);
  return fallback.getTime();
}
