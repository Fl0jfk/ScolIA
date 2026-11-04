import { parisDateKey } from "@/app/lib/paris-time";

export const PERSONNEL_MIN_AGE_YEARS = 14;
export const PERSONNEL_MIN_BIRTH_YEAR = 1900;

export type BirthDateCheck = { ok: true; value: string } | { ok: false; error: string };

/**
 * Valide une date de naissance de personnel : AAAA-MM-JJ réel, pas dans le futur,
 * année >= 1900, âge >= 14 ans. `todayIso` injectable pour les tests.
 */
export function validatePersonnelBirthDate(raw: unknown, todayIso?: string): BirthDateCheck {
  const s = typeof raw === "string" ? raw.trim() : "";
  if (!s) return { ok: false, error: "La date de naissance est obligatoire." };
  const m = s.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!m) return { ok: false, error: "Date de naissance invalide (format AAAA-MM-JJ attendu)." };
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const d = Number(m[3]);
  const dt = new Date(Date.UTC(y, mo - 1, d));
  if (dt.getUTCFullYear() !== y || dt.getUTCMonth() !== mo - 1 || dt.getUTCDate() !== d) {
    return { ok: false, error: "Date de naissance inexistante." };
  }
  if (y < PERSONNEL_MIN_BIRTH_YEAR) {
    return { ok: false, error: `Date de naissance invalide (avant ${PERSONNEL_MIN_BIRTH_YEAR}).` };
  }
  const today = todayIso ?? parisDateKey(new Date());
  if (s > today) return { ok: false, error: "La date de naissance ne peut pas être dans le futur." };
  const ty = Number(today.slice(0, 4));
  const limit = `${String(ty - PERSONNEL_MIN_AGE_YEARS).padStart(4, "0")}${today.slice(4)}`;
  if (s > limit) {
    return { ok: false, error: `Le collaborateur doit avoir au moins ${PERSONNEL_MIN_AGE_YEARS} ans.` };
  }
  return { ok: true, value: s };
}
