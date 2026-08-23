/** Politique de mot de passe ScolIA (compte intranet). */
export const PASSWORD_MIN_LENGTH = 12;

const UPPER = /[A-ZÀ-Ÿ]/;
const LOWER = /[a-zà-ÿ]/;
const DIGIT = /[0-9]/;
const SYMBOL = /[^A-Za-zÀ-ÿ0-9\s]/;

export type PasswordPolicyResult =
  | { ok: true }
  | { ok: false; error: string };

export function validatePasswordPolicy(password: string): PasswordPolicyResult {
  if (password.length < PASSWORD_MIN_LENGTH) {
    return {
      ok: false,
      error: `Le mot de passe doit contenir au moins ${PASSWORD_MIN_LENGTH} caractères.`,
    };
  }
  if (!UPPER.test(password)) {
    return { ok: false, error: "Le mot de passe doit contenir au moins une majuscule." };
  }
  if (!LOWER.test(password)) {
    return { ok: false, error: "Le mot de passe doit contenir au moins une minuscule." };
  }
  if (!DIGIT.test(password)) {
    return { ok: false, error: "Le mot de passe doit contenir au moins un chiffre." };
  }
  if (!SYMBOL.test(password)) {
    return {
      ok: false,
      error: "Le mot de passe doit contenir au moins un symbole (!@#$%…).",
    };
  }
  return { ok: true };
}

export const PASSWORD_POLICY_HINT =
  `Min. ${PASSWORD_MIN_LENGTH} caractères, avec majuscule, minuscule, chiffre et symbole.`;
