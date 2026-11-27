import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { decodeAuthSnapshot, encodeAuthSnapshot, type ScolaAuthSnapshot } from "@/app/lib/auth-snapshot";

const previous = process.env.BETTER_AUTH_SECRET;
process.env.BETTER_AUTH_SECRET = "secret-de-test-uniquement-32-caracteres";
after(() => {
  if (previous === undefined) delete process.env.BETTER_AUTH_SECRET;
  else process.env.BETTER_AUTH_SECRET = previous;
});

const snapshot: ScolaAuthSnapshot = {
  authUserId: "u1",
  userId: "u1",
  email: "prof@exemple.test",
  etablissementId: "e1",
  roles: ["professeur"],
  orgAdmin: false,
  platformAdmin: false,
  twoFactorEnabled: true,
  hasPasskey: false,
};
const forged = { ...snapshot, roles: ["admin", "master"], orgAdmin: true, platformAdmin: true };
const base64 = (value: unknown) => Buffer.from(JSON.stringify(value)).toString("base64url");

test("la valeur posée par le proxy est relue à l'identique", () => {
  const decoded = decodeAuthSnapshot(encodeAuthSnapshot(snapshot));
  assert.equal(decoded?.authUserId, "u1");
  assert.deepEqual(decoded?.roles, ["professeur"]);
  assert.equal(decoded?.platformAdmin, false);
});

test("une identité inventée sans signature est ignorée", () => {
  assert.equal(decodeAuthSnapshot(base64(forged)), null);
  assert.equal(decodeAuthSnapshot(base64({ ...forged, exp: Date.now() + 60_000 })), null);
  assert.equal(decodeAuthSnapshot(`${base64(forged)}.${base64("x")}`), null);
});

test("modifier le contenu d'une valeur signée l'invalide", () => {
  const [, signature] = encodeAuthSnapshot(snapshot)!.split(".");
  assert.equal(decodeAuthSnapshot(`${base64({ ...forged, exp: Date.now() + 60_000 })}.${signature}`), null);
});

test("une signature faite avec un autre secret est refusée", () => {
  const signed = encodeAuthSnapshot(snapshot);
  const secret = process.env.BETTER_AUTH_SECRET;
  process.env.BETTER_AUTH_SECRET = "un-autre-secret-de-test-32-caracteres";
  assert.equal(decodeAuthSnapshot(signed), null);
  process.env.BETTER_AUTH_SECRET = secret;
});

test("une valeur expirée est refusée", () => {
  const signed = encodeAuthSnapshot(snapshot, 1_000);
  assert.ok(decodeAuthSnapshot(signed, 2_000));
  assert.equal(decodeAuthSnapshot(signed, 1_000 + 6 * 60 * 1000), null);
});

test("sans secret, rien n'est transmis ni accepté", () => {
  const signed = encodeAuthSnapshot(snapshot);
  const secret = process.env.BETTER_AUTH_SECRET;
  delete process.env.BETTER_AUTH_SECRET;
  assert.equal(encodeAuthSnapshot(snapshot), null);
  assert.equal(decodeAuthSnapshot(signed), null);
  process.env.BETTER_AUTH_SECRET = secret;
});

test("le proxy efface l'en-tête reçu avant de transmettre la requête", () => {
  const source = readFileSync("proxy.ts", "utf8");
  const removal = source.indexOf("requestHeaders.delete(SCOLA_AUTH_SNAPSHOT_HEADER)");
  assert.ok(removal > 0 && removal < source.indexOf("requestHeaders.set(SCOLA_AUTH_SNAPSHOT_HEADER"));
  // Un seul point de transmission : tout nouvel appel doit passer par nextWithTenant.
  assert.equal(source.match(/NextResponse\.(next|rewrite)\(/g)?.length, 1);
});
