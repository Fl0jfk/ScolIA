import assert from "node:assert/strict";
import test from "node:test";
import {
  buildCommuneRecap,
  communeRecapToCsv,
  COMMUNE_NON_RENSEIGNEE,
} from "@/app/lib/travels-commune-recap";

const p = (key: string, nom: string, commune: string | null) => ({ key, nom, prenom: "A", commune });

test("regroupe par commune sans tenir compte des accents ni de la casse", () => {
  const r = buildCommuneRecap(
    [
      {
        tripId: "t1",
        title: "Londres",
        dateLabel: "01/10/2026",
        participants: [p("1", "DUPONT", "Le Mesnil-Lénard"), p("2", "DURAND", "LE MESNIL LENARD"), p("3", "MARTIN", null)],
      },
    ],
    "2026-2027",
  );
  assert.equal(r.communes.length, 2);
  assert.equal(r.communes[0].commune, "Le Mesnil-Lénard");
  assert.equal(r.communes[0].nbEleves, 2);
  assert.equal(r.communes[1].commune, COMMUNE_NON_RENSEIGNEE);
});

test("un élève parti à deux voyages est compté une fois, deux participations", () => {
  const trips = ["t1", "t2"].map((id) => ({
    tripId: id,
    title: id,
    dateLabel: "",
    participants: [p("1", "DUPONT", "Rouen")],
  }));
  const r = buildCommuneRecap(trips, "2026-2027");
  assert.equal(r.nbEleves, 1);
  assert.equal(r.nbParticipations, 2);
  assert.match(communeRecapToCsv(r), /Rouen;DUPONT;A;;t1;/);
});
