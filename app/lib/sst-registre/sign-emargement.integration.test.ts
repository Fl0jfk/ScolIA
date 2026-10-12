/**
 * Intégration `signSstEmargement` — TEST_DATABASE_URL uniquement.
 */
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import { beginTestDatabase, endTestDatabase } from "@/app/lib/test-database-harness";
import { SST_ATTESTATION_ONLY_DATA_URL } from "@/app/lib/sst-registre/emarger-body";
import { SST_DRAWN_SIGNATURE_TEST_DATA_URL } from "@/app/lib/sst-registre/emarger-test-fixtures";

test("signSstEmargement — attestation et marqueur PNG", async (t) => {
  if (!beginTestDatabase(t)) return;

  const { signSstEmargement } = await import("@/app/lib/sst-registre/db");
  const { getDb, closeDb } = await import("@/db/index");
  const { etablissement } = await import("@/db/schema");

  const db = getDb();
  const [etab] = await db
    .insert(etablissement)
    .values({
      slug: `sst-emarg-${randomUUID().slice(0, 8)}`,
      name: "Test SST émargement",
      dataBucket: "scola-dev",
    })
    .returning({ id: etablissement.id });

  const base = {
    etablissementId: etab.id,
    firstName: "Test",
    lastName: "Agent",
    roles: ["Professeur"],
  };

  try {
    const mismatchedAttestation = await signSstEmargement({
      ...base,
      userId: randomUUID(),
      attestationOnly: true,
      signatureDataUrl: SST_DRAWN_SIGNATURE_TEST_DATA_URL,
    });
    assert.equal(mismatchedAttestation.ok, false);
    if (mismatchedAttestation.ok) return;
    assert.match(mismatchedAttestation.error, /Attestation invalide/i);

    const markerAsDrawn = await signSstEmargement({
      ...base,
      userId: randomUUID(),
      attestationOnly: false,
      signatureDataUrl: SST_ATTESTATION_ONLY_DATA_URL,
    });
    assert.equal(markerAsDrawn.ok, false);
    if (markerAsDrawn.ok) return;
    assert.match(markerAsDrawn.error, /attestation|manuscrite|réservé/i);

    const attestationOk = await signSstEmargement({
      ...base,
      userId: randomUUID(),
      attestationOnly: true,
      signatureDataUrl: SST_ATTESTATION_ONLY_DATA_URL,
      remarques: "test attestation",
    });
    assert.equal(attestationOk.ok, true);

    const drawnOk = await signSstEmargement({
      ...base,
      userId: randomUUID(),
      attestationOnly: false,
      signatureDataUrl: SST_DRAWN_SIGNATURE_TEST_DATA_URL,
    });
    assert.equal(drawnOk.ok, true);
  } finally {
    await endTestDatabase(closeDb);
  }
});
