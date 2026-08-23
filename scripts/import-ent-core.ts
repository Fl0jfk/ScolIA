/**
 * Importe le cœur ENT (sites, élèves, roster, personnel) depuis S3 JSON → PostgreSQL.
 *
 * Usage :
 *   npm run ent:import-core -- --tenant=la-providence-nicolas-barre
 */
import { existsSync, readFileSync } from "node:fs";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { parseEstablishmentsFile, type Establishment } from "../app/lib/app-config-schemas";
import { validateElevesJson, type EleveConfig } from "../app/lib/eleves-config";
import {
  ensureCurrentAnneeScolaire,
  replaceElevesInDb,
  replacePersonnelInDb,
  replaceSchoolRosterInDb,
  replaceSitesInDb,
  type EntSchoolRosterConfig,
} from "../app/lib/ent-core-db";
import { ensureEtablissementFromTenant } from "../app/lib/etablissement-db";
import {
  normalizePersonnelRecord,
  PERSONNEL_INDEX_KEY,
  personnelRecordKey,
  type PersonnelIndexEntry,
  type PersonnelRecord,
} from "../app/lib/personnel-types";
import { getDataS3ClientForTenantSlug } from "../app/lib/s3-clients";
import { s3Key } from "../app/lib/s3-path";
import { resolveTenantBySlug } from "../app/lib/tenant-registry";
import { closeDb, isDatabaseConfigured } from "../db/index";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx <= 0) continue;
    const key = trimmed.slice(0, eqIdx);
    let value = trimmed.slice(eqIdx + 1);
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

function argValue(name: string): string | undefined {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit?.slice(prefix.length);
}

async function readTenantJson<T>(slug: string, bucket: string, relativePath: string): Promise<T | null> {
  let client;
  try {
    client = await getDataS3ClientForTenantSlug(slug);
  } catch {
    const { getPlatformS3Client } = await import("../app/lib/s3-clients");
    client = getPlatformS3Client();
  }
  const key = s3Key(relativePath);
  try {
    const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

async function main() {
  const slug = argValue("tenant")?.trim();
  if (!slug) {
    console.error("Usage : npm run ent:import-core -- --tenant=<slug>");
    process.exit(1);
  }
  if (!isDatabaseConfigured()) {
    console.error("DATABASE_URL manquante.");
    process.exit(1);
  }

  const tenant = await resolveTenantBySlug(slug);
  let etablissementId: string;
  let bucket: string;

  if (tenant?.dataBucket?.trim()) {
    etablissementId = await ensureEtablissementFromTenant(tenant);
    bucket = tenant.dataBucket.trim();
  } else {
    // Repli : ligne Postgres déjà créée (migration auth) même si le registry local est « default ».
    const { getDb } = await import("../db/index");
    const { etablissement } = await import("../db/schema");
    const { eq } = await import("drizzle-orm");
    const [row] = await getDb()
      .select()
      .from(etablissement)
      .where(eq(etablissement.slug, slug))
      .limit(1);
    if (!row?.dataBucket?.trim()) {
      console.error(
        `Tenant inconnu dans le registry et pas de data_bucket en BDD pour « ${slug} ».`,
      );
      process.exit(1);
    }
    etablissementId = row.id;
    bucket = row.dataBucket.trim();
    console.warn(`[ent:import] registry miss — repli BDD slug=${slug} bucket=${bucket}`);
  }

  console.log(`Tenant ${slug} → etablissement_id=${etablissementId} bucket=${bucket}`);

  const anneeId = await ensureCurrentAnneeScolaire(etablissementId);
  console.log(`Année scolaire courante id=${anneeId}`);

  const estRaw = await readTenantJson<unknown>(slug, bucket, "settings/establishments.json");
  const sites: Establishment[] = estRaw ? parseEstablishmentsFile(estRaw) : [];
  const sitesN = await replaceSitesInDb(etablissementId, sites);
  console.log(`Sites : ${sitesN}`);

  const elevesRaw = await readTenantJson<unknown>(slug, bucket, "eleves.json");
  let eleves: EleveConfig[] = [];
  if (Array.isArray(elevesRaw) && elevesRaw.length > 0) {
    const validated = validateElevesJson(elevesRaw);
    if (!validated.ok) {
      console.error(`eleves.json invalide : ${validated.error}`);
      process.exit(1);
    }
    eleves = validated.eleves;
  }
  const elevesN = await replaceElevesInDb(etablissementId, eleves);
  console.log(`Élèves : ${elevesN}`);

  const rosterRaw = await readTenantJson<{
    updatedAt?: string;
    updatedBy?: string;
    teacherCatalog?: unknown;
    classAssignments?: unknown;
  }>(slug, bucket, "settings/school-roster.json");
  const roster: EntSchoolRosterConfig = {
    updatedAt: rosterRaw?.updatedAt || new Date().toISOString(),
    updatedBy: rosterRaw?.updatedBy,
    teacherCatalog: Array.isArray(rosterRaw?.teacherCatalog)
      ? rosterRaw.teacherCatalog.map(String).map((s) => s.trim()).filter(Boolean)
      : [],
    classAssignments: Array.isArray(rosterRaw?.classAssignments)
      ? rosterRaw.classAssignments
          .map((a) => {
            const o = a as Record<string, unknown>;
            return {
              className: String(o.className ?? "").trim(),
              externalUserId: String(o.externalUserId ?? "").trim(),
              name: String(o.name ?? "").trim(),
              email: String(o.email ?? "").trim().toLowerCase(),
            };
          })
          .filter((a) => a.className && a.externalUserId && a.email)
      : [],
  };
  await replaceSchoolRosterInDb(etablissementId, roster);
  console.log(
    `Roster : catalog=${roster.teacherCatalog.length} assignments=${roster.classAssignments.length}`,
  );

  const index = (await readTenantJson<PersonnelIndexEntry[]>(slug, bucket, PERSONNEL_INDEX_KEY)) ?? [];
  const records: PersonnelRecord[] = [];
  for (const entry of Array.isArray(index) ? index : []) {
    const raw = await readTenantJson<unknown>(slug, bucket, personnelRecordKey(entry.id));
    if (!raw) continue;
    try {
      records.push(normalizePersonnelRecord(raw));
    } catch (error) {
      console.warn(`Personnel ${entry.id} ignoré :`, error);
    }
  }
  const personnelN = await replacePersonnelInDb(etablissementId, records);
  console.log(`Personnel : ${personnelN}`);

  console.log("Import cœur ENT terminé.");
  await closeDb();
}

main().catch(async (error) => {
  console.error(error);
  await closeDb().catch(() => undefined);
  process.exit(1);
});
