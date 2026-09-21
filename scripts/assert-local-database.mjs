/**
 * Refuse une URL Postgres non locale, sauf dérogation explicite.
 * Utilisé par les scripts de migration / agents pour ne pas toucher la RDB prod.
 */
export function isLocalDatabaseUrl(url) {
  if (!url || typeof url !== "string") return false;
  try {
    const parsed = new URL(url);
    const host = (parsed.hostname || "").toLowerCase();
    return host === "127.0.0.1" || host === "localhost" || host === "::1";
  } catch {
    return /@127\.0\.0\.1(?:[:/]|$)/.test(url) || /@localhost(?:[:/]|$)/.test(url);
  }
}

/**
 * @param {string | undefined} url
 * @param {{ allowEnv?: string, action?: string }} [opts]
 */
export function assertLocalDatabase(
  url,
  { allowEnv = "ALLOW_PROD_MIGRATION", action = "cette opération" } = {},
) {
  if (isLocalDatabaseUrl(url)) return "local";
  if (process.env[allowEnv] === "1") {
    console.error(
      `[db-guard] ${action} sur une base NON locale autorisée via ${allowEnv}=1 — validation humaine requise.`,
    );
    return "remote-override";
  }
  console.error(
    `[db-guard] Refus : DATABASE_URL n’est pas 127.0.0.1/localhost. ${action} bloquée.`,
  );
  console.error(
    `[db-guard] Tests = Postgres local (install.sh). Prod = merge dev→main après validation humaine.`,
  );
  console.error(
    `[db-guard] Dérogation (prod uniquement, jamais pour un test) : ${allowEnv}=1`,
  );
  process.exit(1);
}
