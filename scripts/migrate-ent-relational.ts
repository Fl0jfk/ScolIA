/**
 * Migre ent_entity + tenant_document → tables relationnelles / collections.
 *
 * Usage:
 *   npm run ent:migrate-relational -- --tenant=la-providence-nicolas-barre
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import { replaceAbsencesInDb } from "../app/lib/absence-db";
import {
  putCollectionSingleton,
  upsertCollectionRecord,
} from "../app/lib/ent-collection-db";
import { jsonPathToCollection } from "../app/lib/ent-json-postgres";
import { replaceTravelsInDb } from "../app/lib/travel-db";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { entEntity, etablissement, tenantDocument } from "../db/schema";
import type { AbsenceRecord } from "../app/lib/absences-types";
import type { TravelsTrip } from "../app/lib/travels-types";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const i = trimmed.indexOf("=");
    if (i < 0) continue;
    const key = trimmed.slice(0, i).trim();
    let val = trimmed.slice(i + 1).trim();
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = val;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

function argValue(name: string): string | null {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit ? hit.slice(prefix.length).trim() : null;
}

async function putDoc(etablissementId: string, relativePath: string, data: unknown) {
  const { collection, recordId, singleton } = jsonPathToCollection(relativePath);
  if (singleton) {
    await putCollectionSingleton(etablissementId, collection, data);
    return;
  }
  if (!data || typeof data !== "object" || Array.isArray(data)) {
    await upsertCollectionRecord(etablissementId, collection, recordId, {
      __root: data as unknown,
    });
    return;
  }
  const obj = { ...(data as Record<string, unknown>) };
  if (obj.id == null) obj.id = recordId;
  await upsertCollectionRecord(etablissementId, collection, recordId, obj);
}

async function main() {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL manquant");
  }
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";

  const slug = argValue("tenant");
  if (!slug) throw new Error("Usage: --tenant=<slug>");

  const db = getDb();
  const [row] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  if (!row) throw new Error(`Établissement inconnu en DB: ${slug}`);
  const etablissementId = row.id;

  console.log(`Migration relationnelle | ${slug} | ${etablissementId}`);

  const entities = await db
    .select()
    .from(entEntity)
    .where(eq(entEntity.etablissementId, etablissementId));

  const absences: AbsenceRecord[] = [];
  const travels: TravelsTrip[] = [];
  let otherEntity = 0;

  for (const entity of entities) {
    const payload = entity.payload as Record<string, unknown>;
    if (entity.kind === "absence") {
      absences.push(payload as unknown as AbsenceRecord);
    } else if (entity.kind === "travel") {
      travels.push(payload as unknown as TravelsTrip);
    } else if (entity.kind === "personnel_leave" || entity.kind.startsWith("domain_")) {
      // agrégés plus bas
    } else {
      otherEntity += 1;
      const path =
        entity.kind === "request"
          ? `requests/${entity.recordId}.json`
          : entity.kind.startsWith("stage_")
            ? `stages/${entity.kind.replace("stage_", "")}s/${entity.recordId}.json`
            : entity.kind.startsWith("certificate_")
              ? `certificates/${entity.kind.replace("certificate_", "")}s/${entity.recordId}.json`
              : `${entity.kind}/${entity.recordId}.json`;
      try {
        await putDoc(etablissementId, path, payload);
      } catch (e) {
        console.warn("  skip entity", entity.kind, entity.recordId, e);
      }
    }
  }

  if (absences.length) {
    const ok: AbsenceRecord[] = [];
    for (const a of absences) {
      try {
        ok.push(a);
        // dry-run normalize via replace one-by-one would be slow; filter invalid dates
        const start = a?.data?.startAt || a?.data?.startDate;
        const end = a?.data?.endAt || a?.data?.endDate;
        if (!start || !end || Number.isNaN(new Date(String(start)).getTime())) {
          console.warn("  skip absence invalid dates", a?.id);
          ok.pop();
        }
      } catch {
        console.warn("  skip absence", a?.id);
      }
    }
    console.log(`  absences typées: ${await replaceAbsencesInDb(etablissementId, ok)}`);
  }
  if (travels.length) {
    console.log(`  travels typés: ${await replaceTravelsInDb(etablissementId, travels)}`);
  }
  console.log(`  autres entités → collections: ${otherEntity}`);

  const leaves = entities.filter((e) => e.kind === "personnel_leave").map((e) => e.payload);
  if (leaves.length) {
    await putDoc(etablissementId, "personnel-ogec/leave-requests.json", leaves);
    console.log(`  personnel_leave: ${leaves.length}`);
  }
  for (const kind of ["domain_domain", "domain_booking", "domain_session", "domain_signup"] as const) {
    const items = entities.filter((e) => e.kind === kind).map((e) => e.payload);
    if (!items.length) continue;
    const file =
      kind === "domain_domain"
        ? "domain-planning/domains.json"
        : kind === "domain_booking"
          ? "domain-planning/bookings.json"
          : kind === "domain_session"
            ? "domain-planning/sessions.json"
            : "domain-planning/signups.json";
    const body = kind === "domain_domain" ? { domains: items } : items;
    await putDoc(etablissementId, file, body);
    console.log(`  ${kind}: ${items.length}`);
  }

  const docs = await db
    .select()
    .from(tenantDocument)
    .where(eq(tenantDocument.etablissementId, etablissementId));

  let mirrored = 0;
  let skipped = 0;
  for (const doc of docs) {
    const key = doc.docKey;
    if (
      key === "absences/index.json" ||
      key.startsWith("absences/") ||
      key === "travels/index.json" ||
      key.startsWith("travels/") ||
      key === "eleves.json" ||
      key === "settings/establishments.json" ||
      key === "settings/school-roster.json"
    ) {
      skipped += 1;
      continue;
    }
    try {
      await putDoc(etablissementId, key, doc.payload);
      mirrored += 1;
    } catch (e) {
      console.warn("  skip doc", key, e);
    }
  }
  console.log(`  tenant_document → collections: ${mirrored} (skip ${skipped})`);
  console.log("OK");
}

main()
  .catch((e) => {
    console.error(e);
    process.exitCode = 1;
  })
  .finally(async () => {
    await closeDb();
  });
