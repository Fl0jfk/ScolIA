/**
 * Smoke test : updateCompanyContactsAfterValidation (tuteur/RH après validation).
 * Usage :
 *   SCOLA_ALLOW_SCRIPT_ETAB=1 NODE_TLS_REJECT_UNAUTHORIZED=0 \
 *     node --require ./scripts/stub-server-only.cjs --import tsx scripts/test-update-company-contacts.mjs
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";

function loadEnvFile(path) {
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
process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
process.env.SCOLA_ALLOW_SCRIPT_ETAB = "1";
process.env.NODE_ENV = process.env.NODE_ENV || "development";

const { getDb, closeDb, isDatabaseConfigured } = await import("../db/index.ts");
const { etablissement } = await import("../db/schema.ts");
const { saveStageConvention, getStageConvention } = await import(
  "../app/lib/stage-storage.ts"
);
const { updateCompanyContactsAfterValidation } = await import(
  "../app/lib/stage-workflow.ts"
);

const CONV_ID = "stg_test_contacts_edit";

function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, "default"))
    .limit(1);
  if (!etab) throw new Error("Établissement default introuvable — npm run seed:dev");

  process.env.SCOLA_SCRIPT_ETABLISSEMENT_ID = etab.id;

  const now = new Date().toISOString();
  const convention = {
    id: CONV_ID,
    schoolYear: "2025-2026",
    status: "signatures_pending",
    internshipKind: "pfmp",
    student: {
      firstName: "Test",
      lastName: "CONTACTS",
      className: "4B",
      level: "4e",
    },
    company: {
      name: "Entreprise Test Contacts",
      address: "2 rue Test",
      postalCode: "76000",
      city: "Rouen",
      activity: "Test",
      tutorName: "Ancien Tuteur",
      tutorEmail: "ancien.tuteur@example.com",
      tutorPhone: "0600000001",
      rhExtraSigner: true,
      rhFirstName: "Old",
      rhLastName: "RH",
      rhEmail: "old.rh@example.com",
    },
    schedule: {
      mode: "uniform_week",
      periodStart: "2026-03-01",
      periodEnd: "2026-03-15",
      days: [
        {
          weekday: 1,
          hasLunchBreak: true,
          morningStart: "09:00",
          morningEnd: "12:00",
          afternoonStart: "13:30",
          afternoonEnd: "17:00",
        },
      ],
      presenceWeekdays: [1, 2, 3, 4, 5],
    },
    teacherReferent: { name: "Prof Test", email: "prof@localhost.dev" },
    signatures: [
      {
        id: "sig_tutor_test",
        role: "tuteur_entreprise",
        label: "Tuteur en entreprise",
        status: "signe",
        signEmail: "ancien.tuteur@example.com",
        signedAt: now,
        signedBy: "Ancien Tuteur",
        signMethod: "code_confirm",
      },
      {
        id: "sig_rh_test",
        role: "rh_entreprise",
        label: "RH entreprise — Old RH",
        status: "en_attente",
        signEmail: "old.rh@example.com",
        signToken: "tok_rh_old_placeholder",
      },
      {
        id: "sig_dir_test",
        role: "direction",
        label: "Direction",
        status: "en_attente",
        signEmail: "admin@localhost.dev",
      },
    ],
    createdAt: now,
    updatedAt: now,
    createdBy: { role: "staff", name: "test-update-company-contacts" },
    history: [{ at: now, by: "test", action: "SEED_TEST" }],
  };

  await saveStageConvention(convention);

  const result = await updateCompanyContactsAfterValidation({
    convention,
    contacts: {
      tutorName: "Nouveau Tuteur",
      tutorEmail: "nouveau.tuteur@example.com",
      tutorPhone: "0600000002",
      rhExtraSigner: true,
      rhFirstName: "New",
      rhLastName: "RH",
      rhEmail: "new.rh@example.com",
    },
    byName: "Admin Test",
  });

  const loaded = await getStageConvention(CONV_ID);
  assert(loaded, "Convention introuvable après update");
  assert(loaded.company.tutorName === "Nouveau Tuteur", `tutorName=${loaded.company.tutorName}`);
  assert(
    loaded.company.tutorEmail === "nouveau.tuteur@example.com",
    `tutorEmail=${loaded.company.tutorEmail}`,
  );
  assert(loaded.company.tutorPhone === "0600000002", `tutorPhone=${loaded.company.tutorPhone}`);
  assert(loaded.company.rhFirstName === "New", `rhFirstName=${loaded.company.rhFirstName}`);
  assert(loaded.company.rhLastName === "RH", `rhLastName=${loaded.company.rhLastName}`);
  assert(loaded.company.rhEmail === "new.rh@example.com", `rhEmail=${loaded.company.rhEmail}`);

  const tutorSig = loaded.signatures.find((s) => s.role === "tuteur_entreprise");
  assert(tutorSig, "Signature tuteur absente");
  assert(tutorSig.status === "en_attente", `tutor status=${tutorSig.status} (doit être annulée)`);
  assert(
    tutorSig.signEmail === "nouveau.tuteur@example.com",
    `tutor signEmail=${tutorSig.signEmail}`,
  );
  assert(Boolean(tutorSig.signToken), "Nouveau token tuteur attendu");

  const rhSig = loaded.signatures.find((s) => s.role === "rh_entreprise");
  assert(rhSig, "Signature RH absente");
  assert(rhSig.status === "en_attente", `rh status=${rhSig.status}`);
  assert(rhSig.signEmail === "new.rh@example.com", `rh signEmail=${rhSig.signEmail}`);
  assert(loaded.status === "signatures_pending", `status=${loaded.status}`);

  const hist = loaded.history.some((h) => h.action === "CONTACTS_ENTREPRISE_MODIFIES");
  assert(hist, "Entrée d'historique CONTACTS_ENTREPRISE_MODIFIES manquante");

  console.log(
    JSON.stringify(
      {
        ok: true,
        workflowOk: result.ok,
        workflowMessage: result.ok ? result.message : result.error,
        tutorStatus: tutorSig.status,
        tutorEmail: loaded.company.tutorEmail,
        rhEmail: loaded.company.rhEmail,
        status: loaded.status,
        resentRoles: result.ok ? result.resentRoles : undefined,
      },
      null,
      2,
    ),
  );
}

main()
  .catch((err) => {
    console.error(err);
    process.exitCode = 1;
  })
  .finally(async () => {
    try {
      await closeDb();
    } catch {
      /* ignore */
    }
  });
