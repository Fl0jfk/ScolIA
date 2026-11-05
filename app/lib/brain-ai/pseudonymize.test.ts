import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createPseudonymizer, type PseudonymizePerson } from "./pseudonymize";

const PEOPLE: PseudonymizePerson[] = [
  { kind: "eleve", firstName: "Léa", lastName: "Martin" },
  { kind: "parent", firstName: "Isabelle", lastName: "Dubois" },
  { kind: "eleve", firstName: "Jean-Baptiste", lastName: "De La Fontaine" },
  { kind: "personnel", firstName: "Pierre", lastName: "Garnier" },
  { kind: "eleve", firstName: "Emma", lastName: "Petit" },
  { kind: "eleve", firstName: "Hugo", lastName: "Lefèvre" },
];

describe("pseudonymize", () => {
  it("aller-retour Prénom Nom", () => {
    const p = createPseudonymizer(PEOPLE);
    const { text, map } = p.mask("Réécris ce message aux parents de Léa Martin.");
    assert.equal(text, "Réécris ce message aux parents de [Élève 1].");
    assert.ok(!text.includes("Martin"));
    assert.equal(p.unmask(text, map), "Réécris ce message aux parents de Léa Martin.");
  });

  it("Nom Prénom inversé et NOM Prénom, forme d'origine conservée", () => {
    const p = createPseudonymizer(PEOPLE);
    const src = "MARTIN Léa et Martin Léa";
    const { text, map } = p.mask(src);
    assert.ok(!/martin/i.test(text));
    assert.equal(p.unmask(text, map), src);
  });

  it("insensible à la casse et aux accents", () => {
    const p = createPseudonymizer(PEOPLE);
    const src = "lea martin et HUGO LEFEVRE";
    const { text, map } = p.mask(src);
    assert.ok(!/lea|martin|hugo|lefevre/i.test(text));
    assert.equal(p.unmask(text, map), src);
  });

  it("noms composés et particules", () => {
    const p = createPseudonymizer(PEOPLE);
    const src = "Jean-Baptiste de la Fontaine, puis Jean Baptiste De La Fontaine";
    const { text, map } = p.mask(src);
    assert.ok(!/fontaine|baptiste/i.test(text));
    assert.equal(p.unmask(text, map), src);
  });

  it("même personne citée deux fois = même étiquette", () => {
    const p = createPseudonymizer(PEOPLE);
    const { text } = p.mask("Léa Martin est absente. Prévenir Léa Martin.");
    assert.equal(text, "[Élève 1] est absente. Prévenir [Élève 1].");
    // stable entre plusieurs textes
    assert.equal(p.mask("Encore Léa Martin").text, "Encore [Élève 1]");
  });

  it("plusieurs personnes, étiquettes par type", () => {
    const p = createPseudonymizer(PEOPLE);
    const { text, map } = p.mask("Isabelle Dubois écrit à Pierre Garnier au sujet de Léa Martin");
    assert.equal(text, "[Parent 1] écrit à [Personnel 1] au sujet de [Élève 1]");
    assert.equal(p.unmask(text, map), "Isabelle Dubois écrit à Pierre Garnier au sujet de Léa Martin");
  });

  it("prénom seul non masqué sans nom ailleurs", () => {
    const p = createPseudonymizer(PEOPLE);
    const src = "Pierre a dit bonjour à Léa.";
    assert.equal(p.mask(src).text, src);
  });

  it("prénom seul masqué si la personne est déjà masquée, et restitué", () => {
    const p = createPseudonymizer(PEOPLE);
    const src = "Léa Martin est malade. Prévenez Léa, et Madame Martin aussi.";
    const { text, map } = p.mask(src);
    assert.ok(!text.includes("Martin") && !text.includes("Léa"));
    assert.equal(p.unmask(text, map), src);
  });

  it("nom de famille seul masqué, mot courant non masqué", () => {
    const p = createPseudonymizer(PEOPLE);
    assert.equal(p.mask("Mme Dubois est venue").text, "Mme [Parent 1 nom] est venue");
    const s2 = "Un petit mot pour Petit";
    assert.equal(p.mask(s2).text, s2);
    // mais le nom complet est masqué
    assert.equal(p.mask("Emma Petit").text, "[Élève 1]");
  });

  it("nom en minuscules non masqué seul (faux positif évité)", () => {
    const p = createPseudonymizer(PEOPLE);
    const s = "le martin pêcheur";
    assert.equal(p.mask(s).text, s);
  });

  it("limites de mots", () => {
    const p = createPseudonymizer(PEOPLE);
    const s = "Martinique et Martinet, Garnierie";
    assert.equal(p.mask(s).text, s);
  });

  it("texte sans nom inchangé, liste vide inchangée", () => {
    const p = createPseudonymizer(PEOPLE);
    const s = "Bonjour, la réunion a lieu mardi à 18h.";
    assert.equal(p.mask(s).text, s);
    assert.equal(createPseudonymizer([]).mask("Léa Martin").text, "Léa Martin");
  });

  it("démasquage tolérant aux variations du modèle", () => {
    const p = createPseudonymizer(PEOPLE);
    const { map } = p.mask("Léa Martin");
    assert.equal(p.unmask("Chère famille de [élève 1] et [ ELEVE  1 ]", map), "Chère famille de Léa Martin et Léa Martin");
    assert.equal(p.unmask("[Élève 9] inconnu", map), "[Élève 9] inconnu");
  });

  it("nom partagé par plusieurs personnes : étiquette neutre ou personne déjà vue", () => {
    const p = createPseudonymizer([
      { kind: "eleve", firstName: "Léa", lastName: "Martin" },
      { kind: "parent", firstName: "Paul", lastName: "Martin" },
    ]);
    const src1 = "Madame Martin";
    const r1 = p.mask(src1);
    assert.ok(!r1.text.includes("Martin"));
    assert.equal(p.unmask(r1.text, r1.map), src1);
    const q = createPseudonymizer([
      { kind: "eleve", firstName: "Léa", lastName: "Martin" },
      { kind: "parent", firstName: "Paul", lastName: "Martin" },
    ]);
    const r2 = q.mask("Paul Martin puis Martin");
    assert.equal(r2.text, "[Parent 1] puis [Parent 1 nom]");
    assert.equal(q.unmask(r2.text), "Paul Martin puis Martin");
  });

  it("performance : plusieurs milliers de personnes", () => {
    const people: PseudonymizePerson[] = [];
    for (let i = 0; i < 6000; i++)
      people.push({ kind: "eleve", firstName: `Prenom${i}x`, lastName: `Nomfamille${i}x` });
    people.push({ kind: "eleve", firstName: "Léa", lastName: "Martin" });
    const t0 = Date.now();
    const p = createPseudonymizer(people);
    const big = "Bonjour Léa Martin, ".repeat(2000);
    const out = p.mask(big);
    assert.ok(!out.text.includes("Martin"));
    assert.ok(Date.now() - t0 < 3000);
  });
});
