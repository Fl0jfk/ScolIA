/**
 * Renvoie le mail de confirmation aux N dernières inscriptions portes ouvertes.
 *
 * Usage:
 *   npx tsx --env-file=.env --require ./scripts/stub-server-only.cjs scripts/resend-po-last-confirmations.ts
 *   npx tsx --env-file=.env --require ./scripts/stub-server-only.cjs scripts/resend-po-last-confirmations.ts --limit=2
 */
import postgres from "postgres";
import { readFileSync, existsSync } from "node:fs";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split(/\r?\n/)) {
    const m = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)=(.*)$/.exec(line.trim());
    if (!m) continue;
    const key = m[1];
    let val = m[2] ?? "";
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    if (!process.env[key]) process.env[key] = val;
  }
}

loadEnvFile(".env");
loadEnvFile(".env.local");

const limitArg = process.argv.find((a) => a.startsWith("--limit="));
const limit = Math.max(1, Math.min(20, Number(limitArg?.split("=")[1] || 2) || 2));

async function main() {
  const url = process.env.DATABASE_URL?.trim();
  if (!url) throw new Error("DATABASE_URL manquant");

  const sql = postgres(url, { max: 1, ssl: "prefer", connect_timeout: 20 });
  const rows = await sql`
    SELECT
      r.id,
      r.etablissement_id,
      r.email,
      r.first_name,
      r.last_name,
      r.slot_id,
      r.slot_label,
      r.slot_start_at,
      r.slot_end_at,
      r.cycle,
      r.phone,
      r.child_first_name,
      r.child_last_name,
      r.classe_souhaitee,
      r.children_info,
      r.consent,
      r.source,
      r.created_at
    FROM portes_ouvertes_registration r
    ORDER BY r.created_at DESC
    LIMIT ${limit}
  `;

  console.log(`Trouvé ${rows.length} inscription(s) :`);
  for (const r of rows) {
    console.log(` - ${r.created_at?.toISOString?.() || r.created_at} | ${r.email} | ${r.first_name} ${r.last_name} | ${r.slot_label}`);
  }

  if (rows.length === 0) {
    await sql.end({ timeout: 5 });
    return;
  }

  // Import après env chargé
  const { buildPortesOuvertesToolPayload } = await import("../app/lib/portes-ouvertes-db");
  const { resendPortesOuvertesVisitorConfirmation } = await import("../app/lib/portes-ouvertes-mail");

  // Résoudre établissement : prendre celui des lignes (mono-tenant typique)
  const etabIds = [...new Set(rows.map((r) => String(r.etablissement_id)))];
  if (etabIds.length !== 1) {
    console.warn(`Plusieurs établissements (${etabIds.join(", ")}) — envoi par ligne.`);
  }

  for (const r of rows) {
    const etabId = String(r.etablissement_id);
    const payload = await buildPortesOuvertesToolPayload(etabId);
    const po = { enabled: true, ...payload };
    const slotFromConfig = po.slots.find((s) => s.id === r.slot_id);
    const startAt = r.slot_start_at
      ? new Date(r.slot_start_at as Date).toISOString()
      : slotFromConfig?.startAt;
    const endAt = r.slot_end_at
      ? new Date(r.slot_end_at as Date).toISOString()
      : slotFromConfig?.endAt;
    if (!startAt || !endAt) {
      console.error(`SKIP ${r.email} — créneau sans horaires`);
      continue;
    }
    const slot = {
      id: String(r.slot_id),
      label: String(r.slot_label || slotFromConfig?.label || "Créneau"),
      startAt,
      endAt,
      cycle: (r.cycle as "ecole" | "college" | "lycee" | null) || slotFromConfig?.cycle,
      maxPlaces: slotFromConfig?.maxPlaces,
    };
    const entry = {
      id: String(r.id),
      slotId: String(r.slot_id),
      slotLabel: slot.label,
      slotStartAt: startAt,
      slotEndAt: endAt,
      firstName: String(r.first_name),
      lastName: String(r.last_name),
      email: String(r.email),
      phone: r.phone ? String(r.phone) : undefined,
      childrenInfo: r.children_info ? String(r.children_info) : undefined,
      childFirstName: r.child_first_name ? String(r.child_first_name) : undefined,
      childLastName: r.child_last_name ? String(r.child_last_name) : undefined,
      cycle: (r.cycle as "ecole" | "college" | "lycee" | undefined) || undefined,
      classeSouhaitee: r.classe_souhaitee ? String(r.classe_souhaitee) : undefined,
      consent: r.consent !== false,
      source: (r.source === "accueil" ? "accueil" : "public") as "public" | "accueil",
      createdAt: new Date(r.created_at as Date).toISOString(),
    };

    const ok = await resendPortesOuvertesVisitorConfirmation({ po, entry, slot });
    console.log(ok ? `OK mail → ${entry.email}` : `ECHEC mail → ${entry.email}`);
  }

  await sql.end({ timeout: 5 });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
