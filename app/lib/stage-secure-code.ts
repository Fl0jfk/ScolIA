import { randomInt } from "crypto";

/** Code à 6 chiffres pour signature par e-mail (évite les liens trop longs). */
export function generateStageSecureCode(): string {
  return String(randomInt(100_000, 1_000_000));
}

export function normalizeSignEmail(email: string): string {
  return email.trim().toLowerCase();
}
