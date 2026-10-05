/**
 * Garde-fou labo hors prod.
 * Autorise Postgres local, ou une URL labo explicite (SCOLA_ENV=lab + ALLOW_LAB_MIGRATION=1
 * + « lab » dans le hostname ou le nom de base). Refuse la RDB prod.
 */
import { isLocalDatabaseUrl } from "./assert-local-database.mjs";

/**
 * @param {string | undefined} url
 */
export function isLabDatabaseUrl(url) {
  if (!url || typeof url !== "string") return false;
  if (isLocalDatabaseUrl(url)) return true;
  try {
    const parsed = new URL(url);
    const host = (parsed.hostname || "").toLowerCase();
    const dbName = (parsed.pathname || "").replace(/^\//, "").split("?")[0].toLowerCase();
    return host.includes("lab") || dbName.includes("lab");
  } catch {
    return /lab/i.test(url);
  }
}

/**
 * @param {string | undefined} url
 * @param {{ action?: string }} [opts]
 */
export function assertLabOrLocalDatabase(url, { action = "cette opération labo" } = {}) {
  if (isLocalDatabaseUrl(url)) return "local";

  const env = (process.env.SCOLA_ENV || "").trim().toLowerCase();
  const allow = process.env.ALLOW_LAB_MIGRATION === "1";

  if (env !== "lab") {
    console.error(
      `[lab-db-guard] Refus : SCOLA_ENV doit être « lab » pour ${action} hors localhost.`,
    );
    process.exit(1);
  }
  if (!allow) {
    console.error(
      `[lab-db-guard] Refus : posez ALLOW_LAB_MIGRATION=1 pour ${action} sur une base labo.`,
    );
    process.exit(1);
  }
  if (!isLabDatabaseUrl(url)) {
    console.error(
      `[lab-db-guard] Refus : l’URL doit contenir « lab » dans le host ou le nom de base (pas la prod).`,
    );
    process.exit(1);
  }

  console.error(
    `[lab-db-guard] ${action} autorisée sur base LABO (SCOLA_ENV=lab, ALLOW_LAB_MIGRATION=1).`,
  );
  return "lab";
}
