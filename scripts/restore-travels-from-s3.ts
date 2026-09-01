/**
 * Restaure les dossiers séjours depuis S3 (JSON complets) vers Postgres.
 * Upsert uniquement — ne supprime pas les dossiers absents de S3.
 *
 * Usage (sur une machine avec accès DATABASE_URL prod) :
 *   npx tsx --env-file=.env.local --require ./scripts/stub-server-only.cjs scripts/restore-travels-from-s3.ts
 *   npx tsx --env-file=.env.local --require ./scripts/stub-server-only.cjs scripts/restore-travels-from-s3.ts --dry-run
 */
import { GetObjectCommand, ListObjectsV2Command, S3Client } from "@aws-sdk/client-s3";
import { loadEnvFile } from "node:process";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { listTravelsFromDb, travelsDbReady, upsertTravelInDb } from "@/app/lib/travel-db";

loadEnvFile(".env.local");

const dryRun = process.argv.includes("--dry-run");
const bucket = process.env.BUCKET_NAME || "docslapro";
const client = new S3Client({
  region: process.env.REGION || "eu-west-3",
  credentials: {
    accessKeyId: process.env.ACCESS_KEY_ID!,
    secretAccessKey: process.env.SECRET_ACCESS_KEY!,
  },
});

async function listTravelJsonKeys(): Promise<string[]> {
  const keys: string[] = [];
  let token: string | undefined;
  do {
    const res = await client.send(
      new ListObjectsV2Command({ Bucket: bucket, Prefix: "travels/", ContinuationToken: token }),
    );
    for (const c of res.Contents || []) {
      const k = c.Key || "";
      if (/^travels\/[0-9a-f-]{36}\.json$/i.test(k) || /^travels\/trip-[0-9]+\.json$/i.test(k)) {
        keys.push(k);
      }
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);
  return keys;
}

async function readTrip(key: string): Promise<TravelsTrip | null> {
  const obj = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
  const body = await obj.Body?.transformToString();
  if (!body) return null;
  const parsed = JSON.parse(body) as TravelsTrip;
  if (!parsed?.id) parsed.id = key.replace(/^travels\//, "").replace(/\.json$/, "");
  return parsed;
}

async function main() {
  const etabId = await travelsDbReady();
  if (!etabId) throw new Error("Postgres travels indisponible (ENT_CORE_DB / DATABASE_URL).");

  const before = await listTravelsFromDb(etabId);
  const beforeIds = new Set(before.map((t) => String(t.id)));
  console.log("Postgres avant:", before.length, "dossier(s)");

  const keys = await listTravelJsonKeys();
  console.log("S3 dossiers complets:", keys.length);

  let restored = 0;
  let missingInDb = 0;
  for (const key of keys) {
    const trip = await readTrip(key);
    if (!trip?.id) {
      console.warn("Ignoré (JSON invalide):", key);
      continue;
    }
    const title = trip.data?.title || trip.id;
    const inDb = beforeIds.has(String(trip.id));
    if (!inDb) missingInDb += 1;
    console.log(`${dryRun ? "[dry-run] " : ""}${inDb ? "MAJ" : "RESTORE"} ${trip.id} | ${trip.status} | ${title}`);
    if (!dryRun) {
      await upsertTravelInDb(etabId, trip);
      restored += 1;
    }
  }

  if (!dryRun) {
    const after = await listTravelsFromDb(etabId);
    console.log("\nPostgres après:", after.length, "dossier(s)");
    console.log("Upsert effectués:", restored, "| manquants avant restauration:", missingInDb);
  } else {
    console.log("\nDry-run terminé — manquants détectés:", missingInDb);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
