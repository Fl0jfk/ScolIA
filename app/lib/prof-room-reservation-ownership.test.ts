import assert from "node:assert/strict";
import { test } from "node:test";
import { canManageRoomReservation } from "@/app/lib/prof-room-reservation-ownership";

const res = { userId: "prof-1", email: "Prof@ecole.fr", bookedByUserId: "admin-1", bookedForOther: true };

test("admin du module : peut tout gérer", () => {
  assert.equal(canManageRoomReservation(res, { userId: "autre" }, true), true);
});

test("bénéficiaire (compte ou e-mail) et réservant : autorisés", () => {
  assert.equal(canManageRoomReservation(res, { userId: "prof-1" }, false), true);
  assert.equal(canManageRoomReservation(res, { userId: "x", email: " prof@ecole.fr " }, false), true);
  assert.equal(canManageRoomReservation(res, { userId: "admin-1" }, false), true);
});

test("autre collègue : refusé", () => {
  assert.equal(canManageRoomReservation(res, { userId: "prof-2", email: "p2@ecole.fr" }, false), false);
  assert.equal(canManageRoomReservation({ email: "" }, { userId: "prof-2", email: "" }, false), false);
  assert.equal(canManageRoomReservation(res, { userId: "" }, false), false);
});
