import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

test("référence d’adresse : HMAC non recalculable sans le secret serveur", async () => {
  process.env.FD_PARENT_EMAIL_REF_SECRET = "secret-de-test";
  const { fdParentEmailRef } = await import("@/app/lib/fiches-dialogue-identity");
  const fiche = "11111111-1111-1111-1111-111111111111";
  const ref = fdParentEmailRef(fiche, "Parent@Example.test");
  assert.match(ref, /^[a-f0-9]{32}$/);
  assert.equal(ref, fdParentEmailRef(fiche, " parent@example.test "), "adresse normalisée");
  assert.notEqual(ref, fdParentEmailRef("22222222-2222-2222-2222-222222222222", "parent@example.test"));
  const naif = createHash("sha256").update(`fd-parent-email:${fiche}:parent@example.test`).digest("hex").slice(0, 32);
  assert.notEqual(ref, naif, "un simple hash ne doit pas suffire");
});
