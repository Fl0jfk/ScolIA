/**
 * Validation locale : doublons N'SONI, Seglas 2nde, Pitte sorti, Berthelot orphelin.
 * Usage :
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --require ./scripts/stub-server-only.cjs --import tsx scripts/validate-stage-roster-dedupe.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { etablissement } from "../db/schema";
import { upsertElevesInDb } from "../app/lib/ent-core-db";
import { invalidateElevesRegistryCache } from "../app/lib/eleves-registry";
import { buildStageClassRoster } from "../app/lib/stage-class-roster";
import { saveStageConvention } from "../app/lib/stage-storage";
import { currentStageSchoolYear, type StageConvention } from "../app/lib/stage-types";
import { valkeyDel } from "../app/lib/valkey";
import { valkeyKeyStagesClassRoster, valkeyKeyStagesConventionsIndex } from "../app/lib/valkey-keys";
import { classKey } from "../app/lib/stage-referents-config";

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

const YEAR = currentStageSchoolYear();
const NOW = new Date().toISOString();

function baseConvention(
  id: string,
  student: StageConvention["student"],
  companyName: string,
): StageConvention {
  return {
    id,
    schoolYear: YEAR,
    status: "signatures_pending",
    internshipKind: "stage_observation",
    createdAt: NOW,
    updatedAt: NOW,
    student,
    company: {
      name: companyName,
      address: "1 rue Test",
      activity: "Services",
      tutorName: "Tuteur",
      tutorEmail: "tuteur@test.local",
    },
    teacherReferent: {
      name: "Prof Validation",
      email: "prof@localhost.dev",
    },
    schedule: {
      mode: "uniform_week",
      periodStart: "2026-06-01",
      periodEnd: "2026-06-05",
      days: [
        {
          weekday: 1,
          hasLunchBreak: true,
          morningStart: "09:00",
          morningEnd: "12:00",
          afternoonStart: "13:30",
          afternoonEnd: "16:30",
        },
      ],
      presenceWeekdays: [1, 2, 3, 4, 5],
    },
    signatures: [],
  };
}

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
  process.env.NODE_ENV = process.env.NODE_ENV || "development";

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, "default"))
    .limit(1);
  if (!etab) throw new Error("Établissement default introuvable");

  process.env.SCOLA_ALLOW_SCRIPT_ETAB = "1";
  process.env.SCOLA_SCRIPT_ETABLISSEMENT_ID = etab.id;

  await upsertElevesInDb(etab.id, [
    {
      ine: "VALNSONI001",
      nom: "N'SONI",
      prenom: "Dane Junior",
      folderName: "N'SONI — Dane Junior — 3A",
      classe: "3A",
      status: "inscrit",
    },
    {
      ine: "VALBERTHE001",
      nom: "BERTHELOT",
      prenom: "Lucas",
      folderName: "BERTHELOT — Lucas — 3A",
      classe: "3A",
      status: "inscrit",
    },
    {
      ine: "VALPITTE001",
      nom: "PITTE",
      prenom: "Sohan",
      folderName: "PITTE — Sohan — 3A",
      classe: "3A",
      status: "ancien",
      dateSortie: "2026-09-01",
    },
    {
      ine: "VALESGLAS001",
      nom: "SEGLAS",
      prenom: "Apolline",
      folderName: "SEGLAS — Apolline — 2A",
      classe: "2A",
      status: "inscrit",
    },
  ]);
  await invalidateElevesRegistryCache(etab.id);

  await saveStageConvention(
    baseConvention(
      "stg_val_nsoni",
      { firstName: "Dane Junior", lastName: "NSONI", className: "3A", level: "3e" },
      "Entreprise Nsoni",
    ),
  );
  await saveStageConvention(
    baseConvention(
      "stg_val_berthelot_eleve",
      { firstName: "Lucas", lastName: "BERTHELOT", className: "3A", level: "3e" },
      "Entreprise Berthelot élève",
    ),
  );
  await saveStageConvention(
    baseConvention(
      "stg_val_berthelot_pere",
      { firstName: "Jean", lastName: "BERTHELOT", className: "3A", level: "3e" },
      "Entreprise Berthelot père?",
    ),
  );
  await saveStageConvention(
    baseConvention(
      "stg_val_pitte",
      { firstName: "Sohan", lastName: "PITTE", className: "3A", level: "3e" },
      "Entreprise Pitte",
    ),
  );
  await saveStageConvention(
    baseConvention(
      "stg_val_seglas",
      { firstName: "Apolline", lastName: "SEGLAS", className: "3A", level: "3e" },
      "Entreprise Seglas",
    ),
  );

  await valkeyDel(
    valkeyKeyStagesConventionsIndex(etab.id),
    valkeyKeyStagesClassRoster(etab.id, YEAR, classKey("3A")),
  );

  const roster = await buildStageClassRoster("3A", YEAR);
  const names = roster.students.map((s) => `${s.nom} ${s.prenom}`);
  console.log(
    JSON.stringify(
      {
        className: roster.className,
        total: roster.summary.total,
        names,
        details: roster.students.map((s) => ({
          nom: s.nom,
          prenom: s.prenom,
          eleveId: Boolean(s.eleveId),
          conventions: s.conventions.map((c) => c.id),
        })),
      },
      null,
      2,
    ),
  );

  const hasNsoniDup =
    names.filter((n) => /n'?soni/i.test(n)).length > 1 ||
    names.some((n) => /^NSONI\b/i.test(n));
  const hasSeglas = names.some((n) => /seglas/i.test(n));
  const hasPitte = names.some((n) => /pitte/i.test(n));
  const hasBerthelotEleve = names.some((n) => /berthelot/i.test(n) && /lucas/i.test(n));
  const hasBerthelotPere = names.some((n) => /berthelot/i.test(n) && /jean/i.test(n));
  const nsoniRow = roster.students.find((s) => /n'?soni/i.test(s.nom));

  const ok =
    !hasNsoniDup &&
    !hasSeglas &&
    !hasPitte &&
    hasBerthelotEleve &&
    hasBerthelotPere &&
    nsoniRow?.nom === "N'SONI";

  console.log(
    JSON.stringify(
      {
        checks: {
          noNsoniDuplicate: !hasNsoniDup,
          nsoniSpellingFromRegistry: nsoniRow?.nom === "N'SONI",
          seglasHidden: !hasSeglas,
          pitteHidden: !hasPitte,
          berthelotEleveKept: hasBerthelotEleve,
          berthelotOrphanKept: hasBerthelotPere,
        },
        ok,
      },
      null,
      2,
    ),
  );

  await closeDb();
  if (!ok) process.exit(1);
}

main().catch(async (err) => {
  console.error(err);
  await closeDb().catch(() => undefined);
  process.exit(1);
});
