import { test } from "node:test";
import assert from "node:assert/strict";
import { anniversaireMonthDays, birthMonthDay, isAnniversaireToday } from "./anniversaires-date";

test("anniversaires — extraction MM-DD", () => {
  assert.equal(birthMonthDay("2010-03-07"), "03-07");
  assert.equal(birthMonthDay("1985-12-25T00:00:00.000Z"), "12-25");
  assert.equal(birthMonthDay("7/3/1990"), "03-07");
  assert.equal(birthMonthDay("n'importe quoi"), null);
  assert.equal(birthMonthDay("2010-13-40"), null);
  assert.equal(birthMonthDay(null), null);
});

test("anniversaires — 29 février fêté le 28 les années non bissextiles", () => {
  assert.deepEqual(anniversaireMonthDays("2027-02-28"), ["02-28", "02-29"]);
  assert.deepEqual(anniversaireMonthDays("2028-02-28"), ["02-28"]);
  assert.deepEqual(anniversaireMonthDays("2028-02-29"), ["02-29"]);
  assert.deepEqual(anniversaireMonthDays("2100-02-28"), ["02-28", "02-29"]);
  assert.equal(isAnniversaireToday("2012-02-29", "2027-02-28"), true);
  assert.equal(isAnniversaireToday("2012-02-29", "2028-02-28"), false);
  assert.equal(isAnniversaireToday("2012-02-29", "2028-02-29"), true);
  assert.equal(isAnniversaireToday("2012-05-01", "2027-05-02"), false);
});

test("anniversaires — maintenance seule : pas d'élèves ; autres rôles internes : oui", async () => {
  const { canSeeEleveAnniversaires, hasInternalRole, primaryInternalRole } = await import("./anniversaires-date");
  assert.equal(canSeeEleveAnniversaires(["maintenance"]), false);
  assert.equal(canSeeEleveAnniversaires(["maintenance", "parent"]), false);
  assert.equal(canSeeEleveAnniversaires(["maintenance", "eleve", "parent"]), false);
  assert.equal(canSeeEleveAnniversaires(["maintenance", "accueil"]), true);
  assert.equal(canSeeEleveAnniversaires(["accueil"]), true);
  assert.equal(canSeeEleveAnniversaires(["professeur"]), true);
  assert.equal(hasInternalRole(["parent", "eleve"]), false);
  assert.equal(hasInternalRole(["parent", "maintenance"]), true);
  assert.equal(primaryInternalRole(["professeur", "direction_college"]), "direction_college");
  assert.equal(primaryInternalRole(["accueil", "professeur"]), "professeur");
  assert.equal(primaryInternalRole(["parent"]), null);
});
