import assert from "node:assert/strict";
import test from "node:test";
import { canDeleteChannelMessage, canManageChannel, canReadChannel } from "@/app/lib/channels-access";

const prof = { userId: "u1", roles: ["professeur"], isAdmin: false };
const admin = { userId: "a", roles: ["admin"], isAdmin: true };

test("salon privé : lisible par les membres et l'admin seulement", () => {
  const c = { id: "x", type: "private", members: ["u2"] };
  assert.equal(canReadChannel(c, prof), false);
  assert.equal(canReadChannel(c, { ...prof, userId: "u2" }), true);
  assert.equal(canReadChannel(c, admin), true);
});

test("salon restreint : rôle requis", () => {
  const c = { id: "x", type: "restricted", roleRequired: "direction" };
  assert.equal(canReadChannel(c, prof), false);
  assert.equal(canReadChannel(c, { ...prof, roles: ["direction"] }), true);
});

test("gestion de salon : créateur ou admin, pas un autre membre du personnel", () => {
  const c = { id: "g", type: "public", createdBy: "u2" };
  assert.equal(canManageChannel(c, prof), false);
  assert.equal(canManageChannel(c, { ...prof, userId: "u2" }), true);
  assert.equal(canManageChannel({ id: "g", type: "public" }, prof), false);
  assert.equal(canManageChannel(c, admin), true);
});

test("suppression de message : auteur ou admin", () => {
  assert.equal(canDeleteChannelMessage({ authorId: "u2" }, prof), false);
  assert.equal(canDeleteChannelMessage({ authorId: "u1" }, prof), true);
  assert.equal(canDeleteChannelMessage({ authorId: null }, prof), false);
  assert.equal(canDeleteChannelMessage({ authorId: null }, admin), true);
});
