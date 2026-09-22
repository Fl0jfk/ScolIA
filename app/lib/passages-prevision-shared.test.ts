/**
 * Prévision repas — helpers purs.
 */
import assert from "node:assert/strict";
import test from "node:test";
import { mealDayKeyFromIsoDate } from "@/app/lib/eleve-grille-repas";
import {
  droitRepasFromRegime,
  repasServiceFromHorodatage,
  resumeFromLignes,
  statutPrevision,
} from "@/app/lib/passages-prevision-shared";

test("mealDayKeyFromIsoDate — mardi 22 sept 2026", () => {
  assert.equal(mealDayKeyFromIsoDate("2026-09-22"), "mar");
  assert.equal(mealDayKeyFromIsoDate("2026-09-20"), null); // dimanche
  assert.equal(mealDayKeyFromIsoDate("2026-09-19"), null); // samedi
  assert.equal(mealDayKeyFromIsoDate("2026-09-21"), "lun");
});

test("droitRepasFromRegime", () => {
  assert.deepEqual(droitRepasFromRegime("Interne"), { midi: true, soir: true });
  assert.deepEqual(droitRepasFromRegime("Demi-pension"), { midi: true, soir: false });
  assert.deepEqual(droitRepasFromRegime("Externe"), { midi: false, soir: false });
  assert.deepEqual(droitRepasFromRegime(null, true), { midi: true, soir: false });
});

test("statutPrevision + résumé", () => {
  assert.equal(statutPrevision(true, true), "attendu_pris");
  assert.equal(statutPrevision(true, false), "attendu_manquant");
  assert.equal(statutPrevision(false, true), "imprevu");
  assert.equal(statutPrevision(false, false), null);

  const resume = resumeFromLignes("2026-09-22", "mar", "Mar", [
    {
      eleveId: "a",
      nom: "A",
      prenom: "A",
      classe: null,
      service: "midi",
      droit: true,
      pris: false,
      statut: "attendu_manquant",
      sourceDroit: "grille",
    },
    {
      eleveId: "b",
      nom: "B",
      prenom: "B",
      classe: null,
      service: "midi",
      droit: false,
      pris: true,
      statut: "imprevu",
      sourceDroit: null,
    },
  ]);
  assert.equal(resume.attendus, 1);
  assert.equal(resume.pris, 1);
  assert.equal(resume.manquants, 1);
  assert.equal(resume.imprevus, 1);
});

test("repasServiceFromHorodatage — coupure 16 h Paris", () => {
  // 09:25 UTC en septembre = 11:25 Paris → midi
  assert.equal(repasServiceFromHorodatage("2026-09-22T09:25:00.000Z"), "midi");
  // 15:00 UTC = 17:00 Paris → soir
  assert.equal(repasServiceFromHorodatage("2026-09-22T15:00:00.000Z"), "soir");
});
