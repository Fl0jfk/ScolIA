/**
 * Remplit first_name / last_name / booked_by_* vides sur reservation_room_booking
 * à partir de la table user (upsert unitaire — jamais de DELETE).
 *
 * Usage: node scripts/backfill-reservation-booking-names.mjs [--dry-run]
 */
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import postgres from "postgres";

function loadEnvFile(filePath) {
  if (!existsSync(filePath)) return;
  for (const line of readFileSync(filePath, "utf8").split("\n")) {
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

loadEnvFile(path.resolve(import.meta.dirname, "../.env.local"));
loadEnvFile(path.resolve(import.meta.dirname, "../.env"));
process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

const dryRun = process.argv.includes("--dry-run");

function splitDisplayName(raw) {
  const parts = String(raw || "")
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (parts.length === 0) return { firstName: "", lastName: "" };
  if (parts.length === 1) {
    const only = parts[0];
    return { firstName: only, lastName: only.toUpperCase() };
  }
  return {
    firstName: parts[0],
    lastName: parts.slice(1).join(" ").toUpperCase(),
  };
}

function resolvePersonNameParts(input) {
  const first = String(input.firstName || "").trim();
  const last = String(input.lastName || "").trim();
  const full = String(input.fullName || input.name || "").trim();
  if (first && last) return { firstName: first, lastName: last.toUpperCase() };
  if (full) {
    const split = splitDisplayName(full);
    return {
      firstName: first || split.firstName,
      lastName: (last || split.lastName).toUpperCase(),
    };
  }
  if (first && !last) return { firstName: first, lastName: first.toUpperCase() };
  if (last) return { firstName: first, lastName: last.toUpperCase() };
  return { firstName: "", lastName: "" };
}

const sql = postgres(process.env.DATABASE_URL, {
  max: 1,
  prepare: false,
  connect_timeout: 60,
  idle_timeout: 10,
  ssl: { rejectUnauthorized: false },
});

try {
  const empty = await sql`
    SELECT
      b.etablissement_id,
      b.id,
      b.user_id,
      b.booked_by_user_id,
      b.email,
      b.first_name,
      b.last_name,
      b.booked_by_first_name,
      b.booked_by_last_name
    FROM reservation_room_booking b
    WHERE b.status <> 'CANCELLED'
      AND (
        COALESCE(TRIM(b.last_name), '') = ''
        OR COALESCE(TRIM(b.booked_by_last_name), '') = ''
        OR (
          COALESCE(TRIM(b.first_name), '') = ''
          AND COALESCE(TRIM(b.last_name), '') = ''
          AND COALESCE(TRIM(b.booked_by_first_name), '') = ''
          AND COALESCE(TRIM(b.booked_by_last_name), '') = ''
        )
      )
  `;

  console.log(`Lignes candidates: ${empty.length}${dryRun ? " (dry-run)" : ""}`);

  let updated = 0;
  let skipped = 0;

  for (const row of empty) {
    const userIds = [row.booked_by_user_id, row.user_id]
      .map((v) => String(v || "").trim())
      .filter(Boolean);

    let userRow = null;
    if (userIds.length) {
      const found = await sql`
        SELECT id, name, first_name, last_name, email, external_user_id
        FROM "user"
        WHERE id = ANY(${userIds})
           OR external_user_id = ANY(${userIds})
        LIMIT 5
      `;
      userRow =
        found.find((u) => u.id === row.booked_by_user_id) ||
        found.find((u) => u.external_user_id === row.booked_by_user_id) ||
        found.find((u) => u.id === row.user_id) ||
        found.find((u) => u.external_user_id === row.user_id) ||
        found[0] ||
        null;
    }

    if (!userRow && row.email) {
      const email = String(row.email).trim().toLowerCase();
      if (email) {
        const byEmail = await sql`
          SELECT id, name, first_name, last_name, email, external_user_id
          FROM "user"
          WHERE lower(email) = ${email}
          LIMIT 1
        `;
        userRow = byEmail[0] || null;
      }
    }

    if (!userRow) {
      skipped += 1;
      continue;
    }

    const resolved = resolvePersonNameParts({
      firstName: userRow.first_name,
      lastName: userRow.last_name,
      name: userRow.name,
    });
    if (!resolved.firstName && !resolved.lastName) {
      skipped += 1;
      continue;
    }

    const nextFirst = String(row.first_name || "").trim() || resolved.firstName;
    const nextLast = String(row.last_name || "").trim() || resolved.lastName;
    const nextByFirst =
      String(row.booked_by_first_name || "").trim() || resolved.firstName;
    const nextByLast =
      String(row.booked_by_last_name || "").trim() || resolved.lastName;

    const changed =
      nextFirst !== String(row.first_name || "") ||
      nextLast !== String(row.last_name || "") ||
      nextByFirst !== String(row.booked_by_first_name || "") ||
      nextByLast !== String(row.booked_by_last_name || "");

    if (!changed) {
      skipped += 1;
      continue;
    }

    console.log(
      `${dryRun ? "[dry] " : ""}update ${row.id}: ` +
        `"${row.first_name}/${row.last_name}" by "${row.booked_by_first_name}/${row.booked_by_last_name}"` +
        ` → "${nextFirst}/${nextLast}" by "${nextByFirst}/${nextByLast}"`,
    );

    if (!dryRun) {
      await sql`
        UPDATE reservation_room_booking
        SET
          first_name = ${nextFirst},
          last_name = ${nextLast},
          booked_by_first_name = ${nextByFirst},
          booked_by_last_name = ${nextByLast},
          updated_at = NOW()
        WHERE etablissement_id = ${row.etablissement_id}
          AND id = ${row.id}
      `;
    }
    updated += 1;
  }

  console.log(`Terminé: ${updated} mises à jour, ${skipped} ignorées.`);
} finally {
  await sql.end({ timeout: 5 });
}
