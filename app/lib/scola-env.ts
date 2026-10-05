/**
 * Environnement runtime ScolIA.
 * `lab` = hors prod (autre Postgres, seed Leo) — bannière visible, jamais main.
 */
export type ScolaRuntimeEnv = "prod" | "lab" | "local";

export function getScolaRuntimeEnv(): ScolaRuntimeEnv {
  const raw = (
    process.env.NEXT_PUBLIC_SCOLA_ENV ||
    process.env.SCOLA_ENV ||
    ""
  )
    .trim()
    .toLowerCase();
  if (raw === "lab" || raw === "labo") return "lab";
  if (raw === "local" || raw === "dev") return "local";
  if (process.env.NODE_ENV !== "production") return "local";
  return "prod";
}

export function isLabRuntime(): boolean {
  return getScolaRuntimeEnv() === "lab";
}
