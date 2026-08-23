/** Politique de mot de passe ScolIA (compte intranet). */
export const PASSWORD_MIN_LENGTH = 12;

const UPPER = /[A-ZÀ-Ÿ]/;
const LOWER = /[a-zà-ÿ]/;
const DIGIT = /[0-9]/;
const SYMBOL = /[^A-Za-zÀ-ÿ0-9\s]/;

export type PasswordPolicyResult =
  | { ok: true }
  | { ok: false; error: string };

export type PasswordRequirementId =
  | "length"
  | "upper"
  | "lower"
  | "digit"
  | "symbol";

export type PasswordRequirement = {
  id: PasswordRequirementId;
  label: string;
  ok: boolean;
};

export function getPasswordRequirements(password: string): PasswordRequirement[] {
  return [
    {
      id: "length",
      label: `Au moins ${PASSWORD_MIN_LENGTH} caractères`,
      ok: password.length >= PASSWORD_MIN_LENGTH,
    },
    {
      id: "upper",
      label: "Une majuscule (A–Z)",
      ok: UPPER.test(password),
    },
    {
      id: "lower",
      label: "Une minuscule (a–z)",
      ok: LOWER.test(password),
    },
    {
      id: "digit",
      label: "Un chiffre (0–9)",
      ok: DIGIT.test(password),
    },
    {
      id: "symbol",
      label: "Un symbole (! ? . @ # $ % …)",
      ok: SYMBOL.test(password),
    },
  ];
}

export function validatePasswordPolicy(password: string): PasswordPolicyResult {
  const missing = getPasswordRequirements(password).find((r) => !r.ok);
  if (!missing) return { ok: true };
  switch (missing.id) {
    case "length":
      return {
        ok: false,
        error: `Le mot de passe doit contenir au moins ${PASSWORD_MIN_LENGTH} caractères.`,
      };
    case "upper":
      return { ok: false, error: "Ajoutez au moins une majuscule (A–Z)." };
    case "lower":
      return { ok: false, error: "Ajoutez au moins une minuscule (a–z)." };
    case "digit":
      return { ok: false, error: "Ajoutez au moins un chiffre (0–9)." };
    case "symbol":
      return {
        ok: false,
        error: "Ajoutez au moins un symbole, par exemple ! ? . @ # $ %",
      };
  }
}

export const PASSWORD_POLICY_HINT =
  `${PASSWORD_MIN_LENGTH} caractères minimum, avec majuscule, minuscule, chiffre et symbole (!?.@#…).`;

export const PASSWORD_POLICY_EXAMPLE = "Exemple : EcoleLpnb2026!";
