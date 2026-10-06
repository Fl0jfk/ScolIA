/**
 * Clôture appel → absence + event métier.
 * npx tsx --test --require ./scripts/stub-server-only.cjs app/lib/vs-appels-close.integration.test.ts
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { randomUUID } from "node:crypto";
import { and, eq } from "drizzle-orm";
import { eleve, etablissement, metierEvent, vsAbsenceEleve, vsAppel, vsAppelLigne } from "@/db/schema";
import { VS_APPEL_EVENT_TYPES } from "@/app/lib/vs-appels-events";

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

const hasDb = Boolean(process.env.DATABASE_URL?.trim());

test("closeAppel — absence source appel + event + motif préservé", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const {
    closeAppel,
    finalizeAppelAbsencesFromLignes,
    reconcileAbsenceWhenLignePresent,
  } = await import("@/app/lib/vs-absences-db");

  const db = getDb();
  const slug = `test-appel-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Test appel", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const [el] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `t:${randomUUID()}`,
      nom: "APPEL",
      prenom: "Test",
      folderName: "APPEL Test",
      classe: "4B",
      status: "inscrit",
    })
    .returning({ id: eleve.id });

  const dateAppel = "2026-10-06";
  const [appel] = await db
    .insert(vsAppel)
    .values({
      etablissementId: etab.id,
      dateAppel,
      classe: "4B",
      heureDebut: "08:00",
      heureFin: "09:00",
      statut: "en_cours",
    })
    .returning();

  try {
    await db.insert(vsAppelLigne).values({
      etablissementId: etab.id,
      appelId: appel.id,
      eleveId: el.id,
      statut: "absent",
      updatedAt: new Date(),
    });
    await finalizeAppelAbsencesFromLignes(etab.id, appel, [
      { eleveId: el.id, statut: "absent" },
    ]);
    const closed = await closeAppel(etab.id, appel.id, { actorUserId: "test-user" });
    assert.ok(closed);
    assert.equal(closed.metierEventType, "attendance.call_completed");
    assert.ok(closed.absenceIds.length >= 1);

    const [absRow] = await db
      .select()
      .from(vsAbsenceEleve)
      .where(
        and(eq(vsAbsenceEleve.etablissementId, etab.id), eq(vsAbsenceEleve.appelId, appel.id)),
      );
    assert.equal(absRow?.source, "appel");

    const [ev] = await db
      .select({ type: metierEvent.type })
      .from(metierEvent)
      .where(
        and(eq(metierEvent.etablissementId, etab.id), eq(metierEvent.aggregateId, appel.id)),
      );
    assert.equal(ev?.type, VS_APPEL_EVENT_TYPES.ATTENDANCE_CALL_COMPLETED);

    await db
      .update(vsAbsenceEleve)
      .set({ motif: "Motif famille test", updatedAt: new Date() })
      .where(eq(vsAbsenceEleve.id, absRow!.id));

    await reconcileAbsenceWhenLignePresent(etab.id, appel.id, el.id);

    const [after] = await db
      .select({ motif: vsAbsenceEleve.motif, statut: vsAbsenceEleve.statut })
      .from(vsAbsenceEleve)
      .where(eq(vsAbsenceEleve.id, absRow!.id));
    assert.equal(after?.motif, "Motif famille test");
    assert.equal(after?.statut, "classee");
  } finally {
    try {
      await db.delete(metierEvent).where(eq(metierEvent.etablissementId, etab.id));
    } catch {
      /* table optionnelle sur vieux schémas locaux */
    }
    await db.delete(vsAbsenceEleve).where(eq(vsAbsenceEleve.etablissementId, etab.id));
    await db.delete(vsAppelLigne).where(eq(vsAppelLigne.etablissementId, etab.id));
    await db.delete(vsAppel).where(eq(vsAppel.etablissementId, etab.id));
    await db.delete(eleve).where(eq(eleve.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});

test("finalizeAppelAbsencesFromLignes — crée source appel même si accueil couvre le créneau", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { finalizeAppelAbsencesFromLignes } = await import("@/app/lib/vs-absences-db");

  const db = getDb();
  const slug = `test-appel-accueil-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Test appel accueil cover", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const [el] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `t:${randomUUID()}`,
      nom: "COVER",
      prenom: "Accueil",
      folderName: "COVER Accueil",
      classe: "1 B",
      status: "inscrit",
    })
    .returning({ id: eleve.id });

  const dateAppel = "2026-10-08";
  await db.insert(vsAbsenceEleve).values({
    etablissementId: etab.id,
    eleveId: el.id,
    appelId: null,
    dateDebut: dateAppel,
    dateFin: dateAppel,
    heureDebut: null,
    heureFin: null,
    type: "absence",
    statut: "a_traiter",
    justifie: false,
    source: "accueil",
    motif: "Parents ont appelé l’accueil",
  });

  const [appel] = await db
    .insert(vsAppel)
    .values({
      etablissementId: etab.id,
      dateAppel,
      classe: "1 B",
      heureDebut: "10:00",
      heureFin: "11:00",
      statut: "en_cours",
    })
    .returning();

  try {
    const ids = await finalizeAppelAbsencesFromLignes(etab.id, appel, [
      { eleveId: el.id, statut: "absent" },
    ]);
    assert.equal(ids.length, 1);

    const rows = await db
      .select({ source: vsAbsenceEleve.source, appelId: vsAbsenceEleve.appelId })
      .from(vsAbsenceEleve)
      .where(and(eq(vsAbsenceEleve.etablissementId, etab.id), eq(vsAbsenceEleve.eleveId, el.id)));

    assert.equal(rows.length, 2);
    assert.ok(rows.some((r) => r.source === "accueil"));
    assert.ok(rows.some((r) => r.source === "appel" && r.appelId === appel.id));
  } finally {
    await db.delete(vsAbsenceEleve).where(eq(vsAbsenceEleve.etablissementId, etab.id));
    await db.delete(vsAppelLigne).where(eq(vsAppelLigne.etablissementId, etab.id));
    await db.delete(vsAppel).where(eq(vsAppel.etablissementId, etab.id));
    await db.delete(eleve).where(eq(eleve.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});

test("closeAppel — pas de double absence même créneau (2 appels)", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { finalizeAppelAbsencesFromLignes } = await import("@/app/lib/vs-absences-db");

  const db = getDb();
  const slug = `test-appel2-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Test appel 2", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const [el] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `t:${randomUUID()}`,
      nom: "DUP",
      prenom: "Test",
      folderName: "DUP Test",
      classe: "4B",
      status: "inscrit",
    })
    .returning({ id: eleve.id });

  const dateAppel = "2026-10-07";
  const [appelA] = await db
    .insert(vsAppel)
    .values({
      etablissementId: etab.id,
      dateAppel,
      classe: "4B",
      heureDebut: "10:00",
      heureFin: "11:00",
      statut: "en_cours",
    })
    .returning();
  const [appelB] = await db
    .insert(vsAppel)
    .values({
      etablissementId: etab.id,
      dateAppel,
      classe: "groupe",
      heureDebut: "10:00",
      heureFin: "11:00",
      statut: "en_cours",
    })
    .returning();

  try {
    await finalizeAppelAbsencesFromLignes(etab.id, appelA, [{ eleveId: el.id, statut: "absent" }]);
    await finalizeAppelAbsencesFromLignes(etab.id, appelB, [{ eleveId: el.id, statut: "absent" }]);

    const absences = await db
      .select({ id: vsAbsenceEleve.id, appelId: vsAbsenceEleve.appelId })
      .from(vsAbsenceEleve)
      .where(
        and(
          eq(vsAbsenceEleve.etablissementId, etab.id),
          eq(vsAbsenceEleve.eleveId, el.id),
          eq(vsAbsenceEleve.source, "appel"),
        ),
      );
    assert.equal(absences.length, 1);
    assert.equal(absences[0]?.appelId, appelA.id);
  } finally {
    await db.delete(vsAbsenceEleve).where(eq(vsAbsenceEleve.etablissementId, etab.id));
    await db.delete(vsAppelLigne).where(eq(vsAppelLigne.etablissementId, etab.id));
    await db.delete(vsAppel).where(eq(vsAppel.etablissementId, etab.id));
    await db.delete(eleve).where(eq(eleve.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});
