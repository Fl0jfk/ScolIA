import assert from "node:assert/strict";
import test from "node:test";
import {
  buildRdvInscriptionIcsLocation,
  RDV_ETABLISSEMENT_PHONE,
} from "@/app/lib/rdv-inscription-contact";
import { buildCalendarEventIcs } from "@/app/lib/calendar-ics";

test("ICS location utilise le téléphone établissement, pas un 06 parent", () => {
  const loc = buildRdvInscriptionIcsLocation("");
  assert.match(loc, /02 32 86 50 90/);
  assert.doesNotMatch(loc, /06 00/);
  assert.equal(RDV_ETABLISSEMENT_PHONE, "02 32 86 50 90");

  const ics = buildCalendarEventIcs({
    title: "RDV test",
    description: `Contact établissement : ${RDV_ETABLISSEMENT_PHONE}`,
    location: loc,
    startAt: "2026-10-02T14:30:00.000Z",
    endAt: "2026-10-02T15:30:00.000Z",
    uid: "rdv-test@scola",
  });
  assert.match(ics, /LOCATION:.*02 32 86 50 90/);
  assert.match(ics, /Contact établissement : 02 32 86 50 90/);
  assert.doesNotMatch(ics, /0600000099/);
});

test("ICS conserve une adresse custom et y ajoute le tél. établissement", () => {
  const loc = buildRdvInscriptionIcsLocation("Accueil lycée");
  assert.equal(loc, `Accueil lycée — Tél. ${RDV_ETABLISSEMENT_PHONE}`);
});
