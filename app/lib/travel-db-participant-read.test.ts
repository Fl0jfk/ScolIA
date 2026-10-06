import assert from "node:assert/strict";
import test from "node:test";
import { isPgUndefinedColumnError } from "./travel-db-pg-errors";

test("isPgUndefinedColumnError — détecte 42703 eleve_id", () => {
  const err = {
    cause: {
      code: "42703",
      message: 'column travel_participant.eleve_id does not exist',
    },
  };
  assert.equal(isPgUndefinedColumnError(err, "eleve_id"), true);
  assert.equal(isPgUndefinedColumnError(err, "other"), false);
});
