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
    description: "Rendez-vous d’inscription (Lycée).",
    location: loc,
    startAt: "2026-10-02T14:30:00.000Z",
    endAt: "2026-10-02T15:30:00.000Z",
    uid: "rdv-test@scola",
  });
  // Les lignes ICS sont pliées à ~75 car. : on regarde le contenu déplié.
  const unfolded = ics.replace(/\r\n[ \t]/g, "");
  assert.match(unfolded, /LOCATION:.*Tél\. 02 32 86 50 90/);
  const phoneHits = unfolded.match(/02 32 86 50 90/g) || [];
  assert.equal(phoneHits.length, 1, `téléphone doit apparaître 1 fois (got ${phoneHits.length})`);
  assert.doesNotMatch(unfolded, /Contact établissement/);
  assert.doesNotMatch(unfolded, /0600000099/);
});

test("ICS conserve une adresse custom et y ajoute le tél. établissement", () => {
  const loc = buildRdvInscriptionIcsLocation("Accueil lycée");
  assert.equal(loc, `Accueil lycée — Tél. ${RDV_ETABLISSEMENT_PHONE}`);
});

test("ICS n’ajoute pas le tél. si déjà présent dans le lieu", () => {
  const loc = buildRdvInscriptionIcsLocation(
    `6, rue de Neuvillette — Tél. ${RDV_ETABLISSEMENT_PHONE}`,
  );
  assert.equal(loc, `6, rue de Neuvillette — Tél. ${RDV_ETABLISSEMENT_PHONE}`);
  const hits = loc.match(/02 32 86 50 90/g) || [];
  assert.equal(hits.length, 1);
});
