import assert from "node:assert/strict";
import test from "node:test";
import { mergeElevesLists } from "./eleves-import";
import { isEleveActifPourListes } from "./eleve-actif-shared";
import { parseSiecleElevesXmlServer } from "./siecle-eleves-parse";

const PAST_SORTIE = "2025-06-30";
const FUTURE_SORTIE = "2027-06-30";

function assertActifInscrit(e: { status?: string; dateSortie?: string } | undefined) {
  assert.equal(e?.status, "inscrit");
  assert.equal(isEleveActifPourListes(e), true);
}

test("réimport Excel inscrit en classe réactive un ancien (vide date_sortie)", () => {
  const existing = [
    {
      ine: "111",
      nom: "BECIANI",
      prenom: "Thomas",
      folderName: "BECIANI Thomas",
      classe: "3A",
      status: "ancien" as const,
      dateSortie: PAST_SORTIE,
    },
  ];
  const incoming = [
    {
      ine: "111",
      nom: "BECIANI",
      prenom: "Thomas",
      folderName: "BECIANI Thomas",
      classe: "4B",
      status: "inscrit" as const,
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming, {
    allowClearDateSortieOnReimport: true,
  });
  assertActifInscrit(eleves[0]);
  assert.equal(eleves[0]?.dateSortie, undefined);
});

test("réimport Excel sans colonne date — sortie passée → réactivation (statut inscrit)", () => {
  const existing = [
    {
      ine: "333",
      nom: "MARTIN",
      prenom: "Paul",
      folderName: "MARTIN Paul",
      classe: "3A",
      status: "ancien" as const,
      dateSortie: PAST_SORTIE,
    },
  ];
  const incoming = [
    {
      ine: "333",
      nom: "MARTIN",
      prenom: "Paul",
      folderName: "MARTIN Paul",
      classe: "4B",
      status: "inscrit" as const,
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming, { dateSortieColumnInFile: false });
  assertActifInscrit(eleves[0]);
  assert.equal(eleves[0]?.dateSortie, undefined);
});

test("import Excel sans colonne date de sortie conserve une date future et le statut inscrit", () => {
  const existing = [
    {
      ine: "334",
      nom: "MARTIN",
      prenom: "Julie",
      folderName: "MARTIN Julie",
      classe: "3A",
      status: "inscrit" as const,
      dateSortie: FUTURE_SORTIE,
    },
  ];
  const incoming = [
    {
      ine: "334",
      nom: "MARTIN",
      prenom: "Julie",
      folderName: "MARTIN Julie",
      classe: "4B",
      status: "inscrit" as const,
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming, { dateSortieColumnInFile: false });
  assertActifInscrit(eleves[0]);
  assert.equal(eleves[0]?.dateSortie, FUTURE_SORTIE);
});

test("réimport Siècle inscrit en classe réactive sans opts explicites", () => {
  const existing = [
    {
      ine: "INE-SIECLE",
      nom: "DURAND",
      prenom: "Léa",
      folderName: "DURAND Léa",
      classe: "3A",
      status: "ancien" as const,
      dateSortie: PAST_SORTIE,
    },
  ];
  const incoming = [
    {
      ine: "INE-SIECLE",
      nom: "DURAND",
      prenom: "Léa",
      folderName: "DURAND Léa",
      classe: "4B",
      status: "inscrit" as const,
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming, { replaceRegime: true });
  assertActifInscrit(eleves[0]);
});

test("réimport internat (merge roster) réactive un sorti passé", () => {
  const existing = [
    {
      ine: "INE-INT",
      nom: "PETIT",
      prenom: "Noé",
      folderName: "PETIT Noé",
      classe: "3A",
      status: "ancien" as const,
      dateSortie: PAST_SORTIE,
      regime: "Externe",
    },
  ];
  const incoming = [
    {
      ine: "INE-INT",
      nom: "PETIT",
      prenom: "Noé",
      folderName: "PETIT Noé",
      classe: "4B",
      status: "inscrit" as const,
      regime: "Interne",
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming, { replaceRegime: true });
  assertActifInscrit(eleves[0]);
});

test("réimport avec date de sortie future garde inscrit", () => {
  const existing = [
    {
      ine: "222",
      nom: "DUPONT",
      prenom: "Marie",
      folderName: "DUPONT Marie",
      classe: "4B",
      status: "ancien" as const,
      dateSortie: PAST_SORTIE,
    },
  ];
  const incoming = [
    {
      ine: "222",
      nom: "DUPONT",
      prenom: "Marie",
      folderName: "DUPONT Marie",
      classe: "4B",
      status: "inscrit" as const,
      dateSortie: FUTURE_SORTIE,
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming);
  assertActifInscrit(eleves[0]);
  assert.equal(eleves[0]?.dateSortie, FUTURE_SORTIE);
});

test("parse Siècle — dateSortieColumnInFile si balise DATE_SORTIE présente", () => {
  const xml = `<?xml version="1.0"?><BEE_ELEVES>
  <ELEVE ELEVE_ID="1"><NOM>DUPONT</NOM><PRENOM>Jean</PRENOM><ID_NATIONAL>100000000AA</ID_NATIONAL>
  <CODE_STRUCTURE>3A</CODE_STRUCTURE><DATE_SORTIE>30/06/2027</DATE_SORTIE></ELEVE>
  </BEE_ELEVES>`;
  const parsed = parseSiecleElevesXmlServer(xml, new Date("2026-10-06T12:00:00+02:00"));
  assert.equal(parsed.dateSortieColumnInFile, true);
  assert.equal(parsed.eleves[0]?.status, "inscrit");
});
