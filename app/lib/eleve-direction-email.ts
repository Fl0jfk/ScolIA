/**
 * Adresses institutionnelles (CE / RNE) qui ne doivent jamais figurer
 * comme e-mail personnel d’un élève (ex. ce.0762565a@ac-normandie.fr).
 *
 * Format académique courant : [ce.]XXXXXXXL@ac-….fr (UAI 7 chiffres + lettre).
 */

const RNE_CE_EMAIL_RE = /^(ce\.)?[0-9]{7}[a-z]@ac-[a-z0-9.-]+\.fr$/i;

/** Normalise pour comparaison (trim + minuscules). */
export function normalizeEmailAddress(raw: string | null | undefined): string {
  return String(raw ?? "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "");
}

/**
 * True si l’adresse est un mail d’établissement (CE / RNE), pas un mail élève.
 */
export function isEstablishmentDirectionEmail(
  raw: string | null | undefined,
): boolean {
  const email = normalizeEmailAddress(raw);
  if (!email) return false;
  return RNE_CE_EMAIL_RE.test(email);
}

/**
 * Retourne l’e-mail s’il est utilisable comme mail élève, sinon `undefined`.
 */
export function sanitizeElevePersonalEmail(
  raw: string | null | undefined,
): string | undefined {
  const email = normalizeEmailAddress(raw);
  if (!email) return undefined;
  if (isEstablishmentDirectionEmail(email)) return undefined;
  return email;
}
