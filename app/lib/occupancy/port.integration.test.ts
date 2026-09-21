/**
 * Intégration occupancy 14.2–14.3 : eleve_id participants + tags présence.
 * Usage : npm run test:occupancy:db
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { randomUUID } from "node:crypto";
import { and, eq } from "drizzle-orm";
import {
  eleve,
  etablissement,
  travel,
  travelParticipant,
  vsAbsenceEleve,
} from "@/db/schema";

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
loadEnvFile("/workspace/.env.local");
loadEnvFile(".env");

const hasDb = Boolean(process.env.DATABASE_URL?.trim());

test("occupancy — en_sortie vs absent_vs, snapshot ≠ live, orphelin", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { occupancy } = await import("@/app/lib/occupancy/port");
  const { resolveEleveIdsByIneKeys } = await import(
    "@/app/lib/travel-participant-resolve"
  );
  const { isExcludedFromBulletinAbsence } = await import("@/app/lib/occupancy/types");

  const db = getDb();
  const slug = `test-occ-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Tenant occupancy test", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const ineOut = `INEOUT${randomUUID().slice(0, 8).toUpperCase()}`;
  const ineSick = `INESICK${randomUUID().slice(0, 8).toUpperCase()}`;

  const [eleveOut] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `test:${randomUUID()}`,
      ine: ineOut,
      nom: "SORTIE",
      prenom: "Alice",
      folderName: "SORTIE Alice",
      status: "inscrit",
      classe: "4B",
    })
    .returning({ id: eleve.id });

  const [eleveSick] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `test:${randomUUID()}`,
      ine: ineSick,
      nom: "MALADE",
      prenom: "Bob",
      folderName: "MALADE Bob",
      status: "inscrit",
      classe: "4B",
    })
    .returning({ id: eleve.id });

  const travelId = `trip-${randomUUID().slice(0, 8)}`;
  await db.insert(travel).values({
    id: travelId,
    etablissementId: etab.id,
    type: "sejour",
    status: "VALIDE",
    title: "Sortie test occupancy",
    startDate: "2026-03-10",
    endDate: "2026-03-12",
    listeElevesStatus: "confirmed",
  });

  const resolved = await resolveEleveIdsByIneKeys({
    etablissementId: etab.id,
    keys: [ineOut, "ORPHELIN999"],
  });
  assert.equal(resolved.get(ineOut), eleveOut.id);
  assert.equal(resolved.has("ORPHELIN999"), false);

  await db.insert(travelParticipant).values([
    {
      etablissementId: etab.id,
      travelId,
      eleveKey: ineOut,
      eleveId: eleveOut.id,
      nom: "SORTIE",
      prenom: "Alice",
      classe: "3A",
      sortOrder: 0,
    },
    {
      etablissementId: etab.id,
      travelId,
      eleveKey: "ORPHELIN999",
      eleveId: null,
      nom: "ORPHELIN",
      prenom: "Zoe",
      classe: "3A",
      sortOrder: 1,
    },
  ]);

  await db.insert(vsAbsenceEleve).values({
    etablissementId: etab.id,
    eleveId: eleveSick.id,
    dateDebut: "2026-03-10",
    dateFin: "2026-03-10",
    type: "absence",
    statut: "a_traiter",
    motif: "Maladie — test occupancy",
    source: "accueil",
  });

  // Même élève en sortie + absence VS inventée → occupancy doit garder en_sortie.
  await db.insert(vsAbsenceEleve).values({
    etablissementId: etab.id,
    eleveId: eleveOut.id,
    dateDebut: "2026-03-10",
    dateFin: "2026-03-10",
    type: "absence",
    statut: "a_traiter",
    motif: "ne doit pas gagner",
    source: "appel",
  });

  try {
    const day = await occupancy({
      etablissementId: etab.id,
      date: "2026-03-10",
      classe: "4B",
    });

    assert.equal(day.unmatchedParticipants.length, 1);
    assert.equal(day.unmatchedParticipants[0]!.eleveKey, "ORPHELIN999");
    assert.equal(day.coverage, "partial");

    const byId = new Map(day.facts.map((f) => [f.eleveId, f]));
    const outFact = byId.get(eleveOut.id);
    assert.ok(outFact);
    assert.equal(outFact!.tag, "en_sortie");
    assert.equal(outFact!.detail?.travelId, travelId);
    assert.equal(outFact!.detail?.snapshotClasse, "3A");
    assert.equal(outFact!.detail?.liveClasse, "4B");
    assert.equal(isExcludedFromBulletinAbsence(outFact!.tag), true);

    const sickFact = byId.get(eleveSick.id);
    assert.ok(sickFact);
    assert.equal(sickFact!.tag, "absent_vs");
    assert.equal(sickFact!.detail?.motif, "Maladie — test occupancy");

    const outside = await occupancy({
      etablissementId: etab.id,
      date: "2026-03-20",
      eleveId: eleveOut.id,
    });
    assert.equal(outside.facts[0]?.tag, "en_cours");
  } finally {
    await db.delete(vsAbsenceEleve).where(eq(vsAbsenceEleve.etablissementId, etab.id));
    await db
      .delete(travelParticipant)
      .where(eq(travelParticipant.etablissementId, etab.id));
    await db.delete(travel).where(eq(travel.etablissementId, etab.id));
    await db.delete(eleve).where(eq(eleve.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});

test("occupancy — élève retiré du voyage → plus en_sortie", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { occupancyFactForEleve } = await import("@/app/lib/occupancy/port");

  const db = getDb();
  const slug = `test-occ2-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Tenant occupancy remove", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const [el] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `test:${randomUUID()}`,
      ine: `INEREM${randomUUID().slice(0, 6).toUpperCase()}`,
      nom: "RETIRE",
      prenom: "Camille",
      folderName: "RETIRE Camille",
      status: "inscrit",
      classe: "5A",
    })
    .returning({ id: eleve.id, ine: eleve.ine });

  const travelId = `trip-${randomUUID().slice(0, 8)}`;
  await db.insert(travel).values({
    id: travelId,
    etablissementId: etab.id,
    type: "sortie",
    status: "VALIDE",
    title: "Retrait participant",
    startDate: "2026-04-01",
    endDate: "2026-04-01",
    listeElevesStatus: "confirmed",
  });

  const [part] = await db
    .insert(travelParticipant)
    .values({
      etablissementId: etab.id,
      travelId,
      eleveKey: el.ine ?? "",
      eleveId: el.id,
      nom: "RETIRE",
      prenom: "Camille",
      classe: "5A",
      sortOrder: 0,
    })
    .returning({ id: travelParticipant.id });

  try {
    const before = await occupancyFactForEleve({
      etablissementId: etab.id,
      date: "2026-04-01",
      eleveId: el.id,
    });
    assert.equal(before?.tag, "en_sortie");

    await db
      .delete(travelParticipant)
      .where(
        and(
          eq(travelParticipant.etablissementId, etab.id),
          eq(travelParticipant.id, part.id),
        ),
      );

    const after = await occupancyFactForEleve({
      etablissementId: etab.id,
      date: "2026-04-01",
      eleveId: el.id,
    });
    assert.equal(after?.tag, "en_cours");
  } finally {
    await db
      .delete(travelParticipant)
      .where(eq(travelParticipant.etablissementId, etab.id));
    await db.delete(travel).where(eq(travel.etablissementId, etab.id));
    await db.delete(eleve).where(eq(eleve.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});
