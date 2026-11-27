import assert from "node:assert/strict";
import test from "node:test";
import {
  matchEleveForInternat,
  parentEmailsFromFamille,
  type InternatEleveCandidate,
  type InternatFamilleResponsable,
} from "./internat-famille-shared";
import {
  filterAppels,
  filterIncidents,
  monthBounds,
  monthCells,
  schoolYearMonthsUpTo,
  schoolYearOf,
  shiftMonth,
} from "./internat-suivi-shared";
import type { InternatIncident } from "./internat-types";

const eleve = (id: string, extra: Partial<InternatEleveCandidate> = {}): InternatEleveCandidate => ({
  id,
  ine: null,
  nom: "DURAND",
  prenom: "Léa",
  folderName: "DURAND — Léa",
  classe: "2A",
  status: "inscrit",
  ...extra,
});

const interne = (ref: Partial<{ ine: string; nom: string; prenom: string }> = {}, classe = "2A") => ({
  classe,
  eleveRef: { folderName: "DURAND — Léa", nom: "DURAND", prenom: "Léa", ...ref },
});

test("famille : l'INE prime, les accents et l'ordre nom / prénom sont ignorés", () => {
  assert.equal(
    matchEleveForInternat(interne({ ine: "abc1" }), [eleve("a"), eleve("b", { ine: "ABC1" })])?.id,
    "b",
  );
  assert.equal(matchEleveForInternat(interne({ nom: "Lea", prenom: "Durand" }), [eleve("a")])?.id, "a");
});

test("famille : la classe passe avant le statut ; ni ancien ni nom inversé d'une autre classe", () => {
  const list = [eleve("bon", { status: "preinscrit" }), eleve("homonyme", { classe: "6B" })];
  assert.equal(matchEleveForInternat(interne(), list)?.id, "bon");
  assert.equal(matchEleveForInternat(interne(), [eleve("a", { classe: "6B", status: "ancien" })]), null);
  assert.equal(
    matchEleveForInternat(interne({ nom: "Lea", prenom: "Durand" }), [eleve("a", { classe: "6B" })]),
    null,
  );
});

test("famille : des homonymes sont départagés par la classe, sinon aucun rattachement", () => {
  const list = [eleve("a", { classe: "2A" }), eleve("b", { classe: "TB" })];
  assert.equal(matchEleveForInternat(interne({}, "TB"), list)?.id, "b");
  assert.equal(matchEleveForInternat(interne({}, "1C"), list), null);
});

test("famille : un INE différent exclut l'élève, un ancien élève passe après l'inscrit", () => {
  assert.equal(matchEleveForInternat(interne({ ine: "X1" }), [eleve("a", { ine: "Y2" })]), null);
  assert.equal(
    matchEleveForInternat(interne(), [eleve("a", { status: "ancien" }), eleve("b")])?.id,
    "b",
  );
});

const resp = (email: string | null, rang: number, autorite = true): InternatFamilleResponsable => ({
  id: `${email}-${rang}`,
  nom: "X",
  prenom: "Y",
  email,
  telephone: null,
  lien: null,
  autoriteParentale: autorite,
  contactUrgence: false,
  peutRecuperer: false,
  rang,
});

test("sorties : parents séparés d'abord, jamais sans autorité parentale, sans doublon", () => {
  const foyer = (id: string, responsables: InternatFamilleResponsable[]) => ({
    id,
    label: id,
    relation: "principal",
    adresse: null,
    codePostal: null,
    ville: null,
    responsables,
  });
  assert.deepEqual(
    parentEmailsFromFamille({
      eleveId: "e",
      foyers: [
        foyer("1", [resp("mere@x.test", 1), resp("beau-pere@x.test", 2, false), resp("MERE@x.test", 3)]),
        foyer("2", [resp("pere@x.test", 1), resp("autre@x.test", 2)]),
      ],
    }),
    ["mere@x.test", "pere@x.test"],
  );
  assert.deepEqual(parentEmailsFromFamille({ eleveId: "e", foyers: [] }), []);
});

test("calendrier : année scolaire, bornes de mois et semaine commençant le lundi", () => {
  assert.equal(schoolYearOf("2026-08-01"), 2026);
  assert.equal(schoolYearOf("2026-07-31"), 2025);
  assert.deepEqual(schoolYearMonthsUpTo("2026-11"), ["2026-08", "2026-09", "2026-10", "2026-11"]);
  assert.equal(schoolYearMonthsUpTo("2027-07").length, 12);
  assert.equal(shiftMonth("2026-12", 1), "2027-01");
  assert.equal(shiftMonth("2027-01", -1), "2026-12");
  assert.deepEqual(monthBounds("2028-02"), { from: "2028-02-01", to: "2028-02-29" });
  // Le 1er novembre 2026 est un dimanche : six cases vides avant.
  assert.equal(monthCells("2026-11").indexOf("2026-11-01"), 6);
});

test("appels et incidents : filtres, recherche sans accents et tri", () => {
  const appels = [
    { date: "2026-11-02", period: "soir" as const, mark: "present" as const, markLabel: "Présent" },
    { date: "2026-12-07", period: "soir" as const, mark: "absent" as const, markLabel: "Absent" },
  ];
  assert.equal(filterAppels(appels, { mark: "all", query: "decembre", ascending: false }).length, 1);
  assert.equal(filterAppels(appels, { mark: "all", query: "02/11", ascending: false }).length, 1);
  assert.equal(filterAppels(appels, { mark: "present", query: "", ascending: false })[0]!.date, "2026-11-02");
  assert.equal(filterAppels(appels, { mark: "all", query: "", ascending: false })[0]!.date, "2026-12-07");

  const inc = (id: string, occurredAt: string, kind: InternatIncident["kind"]): InternatIncident => ({
    id,
    studentId: "s",
    studentName: "S",
    kind,
    title: `Entrée ${id}`,
    occurredAt,
    createdAt: occurredAt,
    createdBy: { userId: "u", name: "Surveillant" },
  });
  const all = [inc("a", "2025-10-01", "sanction"), inc("b", "2026-10-01", "incident")];
  assert.deepEqual(
    filterIncidents(all, { kind: "all", schoolYear: 2026, query: "entree", ascending: false }).map((i) => i.id),
    ["b"],
  );
  assert.deepEqual(
    filterIncidents(all, { kind: "sanction", schoolYear: "all", query: "", ascending: false }).map((i) => i.id),
    ["a"],
  );
});
