/**
 * Garde-fou pour tests / scripts d’intégration : uniquement TEST_DATABASE_URL.
 * Ne jamais lire DATABASE_URL dans un test (risque prod si .env.local chargé).
 */
import { isLocalDatabaseUrl, normalizeDatabaseHostname } from "./assert-local-database.mjs";

/** Hôtes autorisés : machine locale + hostname du service postgres GitHub Actions. */
const ALLOWED_TEST_HOSTS = new Set(["127.0.0.1", "localhost", "::1", "postgres"]);

/** Bases explicitement réservées aux tests automatisés. */
const ALLOWED_TEST_DATABASE_NAMES = new Set([
  "scolia_migrate",
  "scolia_migrate_test",
  "scola_test",
  "scola_integration_test",
]);

const REMOTE_HOST_PATTERN =
  /scaleway|scw\.cloud|amazonaws|neon\.tech|supabase|render\.com|rdb\.|\.fr-par\.|\.nl-ams\./i;

/**
 * @param {string | undefined} url
 * @returns {{ ok: true, url: string } | { ok: false, reason: string }}
 */
export function validateTestDatabaseUrl(url) {
  const trimmed = typeof url === "string" ? url.trim() : "";
  if (!trimmed) {
    return { ok: false, reason: "TEST_DATABASE_URL absente" };
  }

  let parsed;
  try {
    parsed = new URL(trimmed);
  } catch {
    return { ok: false, reason: "TEST_DATABASE_URL invalide" };
  }

  const host = normalizeDatabaseHostname(parsed.hostname);
  if (REMOTE_HOST_PATTERN.test(host)) {
    return { ok: false, reason: "refus : hôte distant (TEST_DATABASE_URL)" };
  }
  if (!ALLOWED_TEST_HOSTS.has(host) && !isLocalDatabaseUrl(trimmed)) {
    return {
      ok: false,
      reason: `refus hôte « ${host} » (127.0.0.1 / localhost / ::1 / postgres CI uniquement)`,
    };
  }

  const dbName = decodeURIComponent((parsed.pathname || "").replace(/^\//, "").split("/")[0] || "");
  const dbLower = dbName.toLowerCase();
  const nameOk =
    ALLOWED_TEST_DATABASE_NAMES.has(dbLower) ||
    dbLower.endsWith("_test") ||
    dbLower.endsWith("_migrate");
  if (!nameOk) {
    return {
      ok: false,
      reason: `refus base « ${dbName} » (nom de base de test explicite requis, ex. scolia_migrate)`,
    };
  }

  return { ok: true, url: trimmed };
}

/** @returns {{ ok: true, url: string } | { ok: false, reason: string }} */
export function resolveTestDatabaseUrl() {
  return validateTestDatabaseUrl(process.env.TEST_DATABASE_URL);
}

/**
 * Branche getDb() / Drizzle en tests : pose DATABASE_URL uniquement depuis TEST_DATABASE_URL validée.
 * @param {{ ok: true, url: string }} validated
 */
export function installTestDatabaseUrlForApp(validated) {
  process.env.DATABASE_URL = validated.url;
}

/** @param {{ ok: false, reason: string } | { ok: true, url: string }} result */
export function testDatabaseSkipReason(result) {
  return result.ok ? null : result.reason;
}

/** Process node:test (`NODE_TEST_CONTEXT`) ou scripts npm (`SCOLIA_TEST_MODE=1`). */
export function isTestProcess() {
  return Boolean(process.env.NODE_TEST_CONTEXT) || process.env.SCOLIA_TEST_MODE === "1";
}

/**
 * En mode test uniquement : refuse DATABASE_URL non conforme (mêmes règles que TEST_DATABASE_URL).
 * @param {string | undefined} url
 */
export function assertDatabaseUrlAllowedInTestProcess(url) {
  if (!isTestProcess()) return;
  const result = validateTestDatabaseUrl(url);
  if (!result.ok) {
    throw new Error(`[db-guard] Connexion refusée en mode test : ${result.reason}`);
  }
}

/**
 * @param {string} dbLower
 * @param {boolean} hostIsLocal
 */
function isAllowedDevScriptDatabaseName(dbLower, hostIsLocal) {
  if (
    ALLOWED_TEST_DATABASE_NAMES.has(dbLower) ||
    dbLower.endsWith("_test") ||
    dbLower.endsWith("_migrate") ||
    dbLower.endsWith("_dev")
  ) {
    return true;
  }
  return hostIsLocal && dbLower === "scola";
}

/**
 * Refuse tout hôte non local (jamais contournable via I_KNOW_THIS_IS_NOT_PROD).
 * @param {string | undefined} url
 * @returns {{ ok: true, url: string, hostIsLocal: true } | { ok: false, reason: string }}
 */
export function validateDevScriptDatabaseHost(url) {
  const trimmed = typeof url === "string" ? url.trim() : "";
  if (!trimmed) {
    return { ok: false, reason: "DATABASE_URL absente" };
  }

  let parsed;
  try {
    parsed = new URL(trimmed);
  } catch {
    return { ok: false, reason: "DATABASE_URL invalide" };
  }

  const host = normalizeDatabaseHostname(parsed.hostname);
  if (REMOTE_HOST_PATTERN.test(host)) {
    return { ok: false, reason: "refus : hôte distant (seed / drizzle-kit push)" };
  }
  const hostIsLocal = ALLOWED_TEST_HOSTS.has(host) || isLocalDatabaseUrl(trimmed);
  if (!hostIsLocal) {
    return {
      ok: false,
      reason: `refus hôte « ${host} » (127.0.0.1 / localhost / ::1 / postgres CI uniquement pour seed et push)`,
    };
  }

  return { ok: true, url: trimmed, hostIsLocal: true };
}

/**
 * Garde seed:dev / drizzle-kit push — hôte local + suffixe test/dev (ou `scola` en local).
 * I_KNOW_THIS_IS_NOT_PROD=1 : ignore uniquement le suffixe de base, jamais le contrôle d’hôte.
 * @param {string | undefined} url
 * @returns {{ ok: true, url: string } | { ok: false, reason: string }}
 */
export function validateDevScriptDatabaseUrl(url) {
  const hostResult = validateDevScriptDatabaseHost(url);
  if (!hostResult.ok) return hostResult;

  const trimmed = hostResult.url;
  let parsed;
  try {
    parsed = new URL(trimmed);
  } catch {
    return { ok: false, reason: "DATABASE_URL invalide" };
  }

  if (process.env.I_KNOW_THIS_IS_NOT_PROD === "1") {
    console.error(
      "[db-guard] I_KNOW_THIS_IS_NOT_PROD=1 — suffixe de base ignoré (hôte local validé, validation humaine).",
    );
    return { ok: true, url: trimmed };
  }

  const dbName = decodeURIComponent((parsed.pathname || "").replace(/^\//, "").split("/")[0] || "");
  const dbLower = dbName.toLowerCase();
  if (!isAllowedDevScriptDatabaseName(dbLower, hostResult.hostIsLocal)) {
    return {
      ok: false,
      reason: `refus base « ${dbName} » (suffixe _test / _migrate / _dev ou base locale scola requis, ou I_KNOW_THIS_IS_NOT_PROD=1 sur hôte local)`,
    };
  }

  return { ok: true, url: trimmed };
}
