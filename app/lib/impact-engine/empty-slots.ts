/**
 * Détection pure : créneau vidé si tous les attendus sont en sortie.
 * Aucune écriture. Population vide → pas de signal (pas d’affectation inventée).
 */

export function isCreneauVide(opts: {
  expectedEleveIds: readonly string[];
  enSortieEleveIds: ReadonlySet<string>;
}): boolean {
  const expected = [...new Set(opts.expectedEleveIds.filter(Boolean))];
  if (expected.length === 0) return false;
  return expected.every((id) => opts.enSortieEleveIds.has(id));
}

export function countEnSortieOnCreneau(opts: {
  expectedEleveIds: readonly string[];
  enSortieEleveIds: ReadonlySet<string>;
}): { expectedCount: number; enSortieCount: number; vide: boolean } {
  const expected = [...new Set(opts.expectedEleveIds.filter(Boolean))];
  const enSortieCount = expected.filter((id) => opts.enSortieEleveIds.has(id)).length;
  return {
    expectedCount: expected.length,
    enSortieCount,
    vide: isCreneauVide(opts),
  };
}

/** Dates civiles inclusives YYYY-MM-DD (UTC calendaire, pas d’heure locale). */
export function eachIsoDateInclusive(start: string, end: string): string[] {
  const s = start.trim();
  const e = (end.trim() || s).trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s) || !/^\d{4}-\d{2}-\d{2}$/.test(e)) return [];
  if (e < s) return [];
  const out: string[] = [];
  const [ys, ms, ds] = s.split("-").map(Number);
  const [ye, me, de] = e.split("-").map(Number);
  const cur = new Date(Date.UTC(ys, ms - 1, ds));
  const last = new Date(Date.UTC(ye, me - 1, de));
  while (cur.getTime() <= last.getTime()) {
    const y = cur.getUTCFullYear();
    const m = String(cur.getUTCMonth() + 1).padStart(2, "0");
    const d = String(cur.getUTCDate()).padStart(2, "0");
    out.push(`${y}-${m}-${d}`);
    cur.setUTCDate(cur.getUTCDate() + 1);
    if (out.length > 366) break;
  }
  return out;
}

export function signalIdFor(opts: {
  etablissementId: string;
  travelId: string;
  date: string;
  creneauId: string;
}): string {
  return `creneau_vide:${opts.etablissementId}:${opts.travelId}:${opts.date}:${opts.creneauId}`;
}
