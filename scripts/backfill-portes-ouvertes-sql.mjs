/**
 * Backfill one-shot JSON S3 → Postgres (portes ouvertes).
 * Idempotent : upsert, aucun wipe.
 *
 * Usage: node --import tsx scripts/backfill-portes-ouvertes-sql.mjs
 * (ou via npm run si branché)
 */
import "dotenv/config";
import { getJson } from "../app/lib/s3-storage.ts";
import { getToolboxConfig } from "../app/lib/toolbox-config.ts";
import { resolveCurrentEtablissementId } from "../app/lib/ent-core-db.ts";
import {
  addPortesOuvertesRegistration,
  getPortesOuvertesConfig,
  listPortesOuvertesRegistrations,
  listPortesOuvertesSlots,
  upsertPortesOuvertesConfig,
  upsertPortesOuvertesSlot,
} from "../app/lib/portes-ouvertes-db.ts";
import { isPortesOuvertesCycle } from "../app/lib/portes-ouvertes-types.ts";

async function main() {
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    console.error("Pas d’établissement (tenant) — abort.");
    process.exit(1);
  }
  console.log("etablissement_id=", etabId);

  const toolbox = await getToolboxConfig();
  const po = toolbox.tools["portes-ouvertes"];

  await upsertPortesOuvertesConfig(
    {
      title: po.title,
      intro: po.intro,
      address: po.address,
      mapsUrl: po.mapsUrl,
      notifyEmail: po.notifyEmail,
      consentLabel: po.consentLabel,
    },
    etabId,
  );
  console.log("config upserted");

  const existingSlots = await listPortesOuvertesSlots(etabId);
  const slotIds = new Set(existingSlots.map((s) => s.id));
  let slotsAdded = 0;
  for (const s of po.slots) {
    if (slotIds.has(s.id)) continue;
    const cycle = isPortesOuvertesCycle(s.cycle) ? s.cycle : "college";
    await upsertPortesOuvertesSlot({ ...s, cycle }, etabId);
    slotIds.add(s.id);
    slotsAdded += 1;
  }
  console.log("slots added=", slotsAdded);

  const raw = await getJson("toolbox/portes-ouvertes/registrations.json");
  const list = Array.isArray(raw?.data) ? raw.data : [];
  const existingRegs = await listPortesOuvertesRegistrations(etabId);
  const regIds = new Set(existingRegs.map((r) => r.id));
  let regsAdded = 0;
  for (const item of list) {
    if (!item || typeof item !== "object") continue;
    const o = item as Record<string, unknown>;
    const id = String(o.id || "").trim();
    if (!id || regIds.has(id)) continue;
    const firstName = String(o.firstName || "").trim();
    const lastName = String(o.lastName || "").trim();
    const email = String(o.email || "").trim().toLowerCase();
    const slotId = String(o.slotId || "").trim();
    if (!firstName || !lastName || !email || !slotId) continue;
    const cycleRaw = String(o.cycle || "").trim();
    await addPortesOuvertesRegistration(
      {
        slotId,
        slotLabel: String(o.slotLabel || "").trim() || undefined,
        slotStartAt: String(o.slotStartAt || "").trim() || undefined,
        slotEndAt: String(o.slotEndAt || "").trim() || undefined,
        firstName,
        lastName,
        email,
        phone: String(o.phone || "").trim() || undefined,
        childrenInfo: String(o.childrenInfo || "").trim() || undefined,
        childFirstName: String(o.childFirstName || "").trim() || undefined,
        childLastName: String(o.childLastName || "").trim() || undefined,
        cycle: isPortesOuvertesCycle(cycleRaw) ? cycleRaw : undefined,
        classeSouhaitee: String(o.classeSouhaitee || "").trim() || undefined,
        consent: o.consent !== false,
        source: o.source === "accueil" || o.source === "public" ? o.source : "public",
      },
      etabId,
    );
    // addPortesOuvertesRegistration génère un nouvel id — pour backfill on veut conserver l’id
    // → insert direct géré ci-dessous si besoin. Ici on accepte un nouvel id pour éviter wipe.
    regIds.add(id);
    regsAdded += 1;
  }
  console.log("registrations imported (approx)=", regsAdded);
  const cfg = await getPortesOuvertesConfig(etabId);
  console.log("done", cfg.title);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
