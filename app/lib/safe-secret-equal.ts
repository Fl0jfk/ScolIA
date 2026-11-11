import { createHash, timingSafeEqual } from "node:crypto";

/** Comparaison de secrets en temps constant (NJ-52, D-053). Faux si l'un des deux est vide. */
export function safeSecretEqual(a: unknown, b: unknown): boolean {
  const x = typeof a === "string" ? a : "";
  const y = typeof b === "string" ? b : "";
  if (!x || !y) return false;
  const hx = createHash("sha256").update(x).digest();
  const hy = createHash("sha256").update(y).digest();
  return timingSafeEqual(hx, hy);
}
