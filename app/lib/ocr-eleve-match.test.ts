import assert from "node:assert/strict";
import test from "node:test";
import type { EleveConfig } from "./eleves-config";
import {
  classifyDocumentOrigin,
  extractInesFromText,
  matchEleveFromDocument,
  scanStudentsInText,
} from "./ocr-eleve-match";

function eleve(partial: Partial<EleveConfig> & Pick<EleveConfig, "nom" | "prenom">): EleveConfig {
  return {
    ine: partial.ine || "",
    nom: partial.nom,
    prenom: partial.prenom,
    folderName: `${partial.nom} ${partial.prenom}`,
    classe: partial.classe,
    dateNaissance: partial.dateNaissance,
  };
}

const roster: EleveConfig[] = [
  eleve({ ine: "1234567890A", nom: "DUPONT", prenom: "Marie", classe: "2A", dateNaissance: "2010-03-15" }),
  eleve({ ine: "1234567890B", nom: "MARTIN", prenom: "Paul", classe: "2A" }),
  eleve({ ine: "9988776655C", nom: "MARTIN", prenom: "Pierre", classe: "3B" }),
  eleve({ ine: "1112223334D", nom: "BERGER", prenom: "Léa", classe: "CM2" }),
];

test("INE exact unique → auto", () => {
  const r = matchEleveFromDocument({
    text: "Bulletin INE 1234567890A DUPONT Marie",
    eleves: roster,
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.matchedBy, "ine");
  assert.equal(r.eleve?.prenom, "Marie");
});

test("INE 1 substitution unique → auto", () => {
  const r = matchEleveFromDocument({
    text: "Identifiant 1234567891A bulletin",
    eleves: roster,
    extracted: { ine: "1234567891A" },
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.matchedBy, "ine_flou");
  assert.equal(r.eleve?.prenom, "Marie");
});

test("INE 2 substitutions → pas d'auto INE", () => {
  const r = matchEleveFromDocument({
    text: "Identifiant 1294567890Z",
    eleves: roster,
    extracted: { ine: "1294567890Z" },
  });
  assert.notEqual(r.matchedBy, "ine");
  assert.notEqual(r.matchedBy, "ine_flou");
});

test("MARTIN Paul vs MARTIN Pierre → pas d'auto si prénoms distincts extraits", () => {
  const r = matchEleveFromDocument({
    text: "Document administratif MARTIN Jean (parent)",
    eleves: roster,
    extracted: { nom: "MARTIN", prenom: "Jacques", origine: "externe" },
  });
  assert.notEqual(r.decision, "auto");
});

test("DUP0NT Marie dans le texte OCR → auto scan", () => {
  const r = matchEleveFromDocument({
    text: "Attestation établie pour DUP0NT Marie née le 15/03/2010",
    eleves: roster,
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.eleve?.prenom, "Marie");
});

test("fratrie même nom, prénoms dans le texte → review", () => {
  const r = matchEleveFromDocument({
    text: "Attestation CAF MARTIN Paul et MARTIN Pierre",
    eleves: roster,
  });
  assert.equal(r.decision, "review");
  assert.ok(r.candidates.length >= 2);
});

test("parent extrait, enfant unique dans le texte → auto enfant", () => {
  const r = matchEleveFromDocument({
    text: "Attestation d'assurance MUTUELLE DUPONT Jean (assuré) pour l'enfant DUPONT Marie",
    eleves: roster,
    extracted: { nom: "DUPONT", prenom: "Jean", origine: "externe" },
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.eleve?.prenom, "Marie");
  assert.equal(r.matchedBy, "scan_texte_enfant");
});

test("nom+prénom+classe interne → auto", () => {
  const r = matchEleveFromDocument({
    text: "Bulletin scolaire 2A",
    eleves: roster,
    extracted: { nom: "DUPONT", prenom: "Marie", classe: "2A", origine: "interne" },
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.eleve?.prenom, "Marie");
});

test("date de naissance confirme un doc externe", () => {
  const r = matchEleveFromDocument({
    text: "Carte nationale d'identité délivrée à l'intéressée",
    eleves: roster,
    extracted: {
      nom: "DUPONT",
      prenom: "Marie",
      dateNaissance: "15/03/2010",
      origine: "externe",
    },
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.eleve?.prenom, "Marie");
  assert.equal(r.matchedBy, "nom_prenom_ddn");
});

test("extractInesFromText lit un INE bruité", () => {
  const ines = extractInesFromText("INE : 123456789OA");
  assert.ok(ines.includes("1234567890A"));
});

test("classifyDocumentOrigin interne/externe", () => {
  assert.equal(classifyDocumentOrigin("Bulletin scolaire Trimestre 1"), "interne");
  assert.equal(classifyDocumentOrigin("Carte nationale d'identité"), "externe");
});

test("scanStudentsInText ignore un nom seul", () => {
  const hits = scanStudentsInText("Le professeur MARTIN signe le document", roster);
  assert.equal(hits.length, 0);
});
