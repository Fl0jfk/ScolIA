/**
 * Garde-fou pour tests / scripts d’intégration : uniquement TEST_DATABASE_URL.
 * Ne jamais lire DATABASE_URL dans un test (risque prod si .env.local chargé).
 */
import { isLocalDatabaseUrl } from "./assert-local-database.mjs";

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

  const host = (parsed.hostname || "").toLowerCase();
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
    dbLower.includes("_migrate");
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
