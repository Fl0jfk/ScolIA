/** Logique de date pure des anniversaires (sans DB, testable). */

function isLeap(year: number): boolean {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

/** Extrait "MM-DD" d'une date de naissance (YYYY-MM-DD, ISO complet ou DD/MM/YYYY). */
export function birthMonthDay(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const s = raw.trim();
  let m = /^(\d{4})-(\d{2})-(\d{2})/.exec(s);
  let mm: number;
  let dd: number;
  if (m) {
    mm = Number(m[2]);
    dd = Number(m[3]);
  } else {
    m = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/.exec(s);
    if (!m) return null;
    dd = Number(m[1]);
    mm = Number(m[2]);
  }
  if (mm < 1 || mm > 12 || dd < 1 || dd > 31) return null;
  return `${String(mm).padStart(2, "0")}-${String(dd).padStart(2, "0")}`;
}

/** Jours "MM-DD" à fêter pour `todayKey` (YYYY-MM-DD) : le 28/02 d'une année non bissextile inclut aussi 02-29. */
export function anniversaireMonthDays(todayKey: string): string[] {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(todayKey);
  if (!m) return [];
  const md = `${m[2]}-${m[3]}`;
  if (md === "02-28" && !isLeap(Number(m[1]))) return ["02-28", "02-29"];
  return [md];
}

export function isAnniversaireToday(birth: string | null | undefined, todayKey: string): boolean {
  const md = birthMonthDay(birth);
  return md !== null && anniversaireMonthDays(todayKey).includes(md);
}

/** Rôles non internes (familles / élèves) : exclus de la liste du personnel. */
const NON_INTERNAL_ROLES = new Set(["parent", "eleve"]);

/** Vrai si au moins un rôle est un rôle interne (tout sauf parent/eleve). */
export function hasInternalRole(roles: readonly string[]): boolean {
  return roles.some((r) => !NON_INTERNAL_ROLES.has(r));
}

/**
 * Maintenance seule (éventuellement avec parent/élève) : pas d'accès à la liste des élèves.
 * Tous les autres rôles internes (accueil compris) voient les élèves.
 */
export function canSeeEleveAnniversaires(roles: readonly string[]): boolean {
  const internal = roles.filter((r) => !NON_INTERNAL_ROLES.has(r));
  return !(internal.length > 0 && internal.every((r) => r === "maintenance"));
}

/** Rôle principal pour l'affichage de la fonction : direction > professeur > premier autre rôle interne. */
export function primaryInternalRole(roles: readonly string[]): string | null {
  const internal = roles.filter((r) => !NON_INTERNAL_ROLES.has(r));
  return (
    internal.find((r) => r === "direction" || r.startsWith("direction_")) ??
    internal.find((r) => r === "professeur") ??
    internal[0] ??
    null
  );
}
