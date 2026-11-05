import { test } from "node:test";
import assert from "node:assert/strict";
import { niveauFromClasse } from "@/app/lib/internat-level";
import { matchFdClasse, niveauxDesCibles } from "@/app/lib/fiches-dialogue-ciblage";
import { presetForNiveau } from "@/app/lib/fiches-dialogue-templates";

test("niveau reconnu pour les divisions nommées par lettre ou numéro", () => {
  const cas: Array<[string, string | null]> = [
    ["6A", "6e"], ["6F", "6e"], ["6E", "6e"], ["5 B", "5e"], ["4C", "4e"],
    ["3A", "3e"], ["3F", "3e"], ["3E", "3e"], ["3EME2", "3e"], ["03 D", "3e"], ["3 4", "3e"],
    ["2A", "2nde"], ["2NDE", "2nde"], ["seconde", "2nde"],
    ["1 A", "1re"], ["1C", "1re"], ["1F", "1re"], ["1ère B", "1re"],
    ["TA", "Tle"], ["TE", "Tle"], ["Terminale C", "Tle"],
    ["C.P MME DEHAYS", null], ["JE1 MME BAYEL", null], ["", null],
  ];
  for (const [classe, attendu] of cas) {
    assert.equal(niveauFromClasse(classe), attendu, `classe « ${classe} »`);
  }
});

test("une cible de niveau couvre toutes les divisions du niveau", () => {
  for (const c of ["6A", "6B", "6C", "6D", "6E", "6F"]) {
    assert.ok(matchFdClasse(c, ["6e"]), c);
  }
  assert.ok(!matchFdClasse("5A", ["6e"]));
});

test("une cible de classe ne prend que cette classe", () => {
  assert.ok(matchFdClasse("3E", ["3E"]));
  assert.ok(matchFdClasse("1 A", ["1A"]));
  assert.ok(!matchFdClasse("3A", ["3E"]), "3E (classe) ne doit pas prendre 3A");
  assert.ok(!matchFdClasse("3EME2", ["3E"]), "plus de correspondance par préfixe");
  assert.ok(matchFdClasse("n'importe", []), "aucune cible = tout le monde");
});

test("détection des campagnes multi-niveaux", () => {
  assert.deepEqual(niveauxDesCibles(["6e", "5e", "4e", "3e"]).sort(), ["3e", "4e", "5e", "6e"]);
  assert.deepEqual(niveauxDesCibles(["3A", "3B", "3E"]), ["3e"]);
});

test("après la 3e : 2nde GT, 2nde pro et CAP proposées", () => {
  const preset = presetForNiveau("3e", "famille_dabord");
  const ids = preset.catalogue.destinations.map((d) => d.id);
  for (const id of ["2nde", "2nde_pro", "cap", "maintien", "ailleurs", "autre_parcours"]) {
    assert.ok(ids.includes(id), id);
  }
  const etab = preset.catalogue.fields?.find((f) => f.id === "etablissements");
  assert.deepEqual(etab?.showWhen?.equals, ["2nde", "2nde_pro", "cap", "ailleurs"]);
  // Les autres niveaux ne changent pas.
  const p4 = presetForNiveau("4e", "conseil_dabord");
  assert.deepEqual(
    p4.catalogue.destinations.map((d) => d.id),
    ["3e", "maintien", "ailleurs", "autre_parcours"],
  );
});

test("rang des niveaux cohérent pour les divisions collège (pilotage)", async () => {
  const { classProgressRank } = await import("@/app/lib/pilotage-eleves-logic");
  assert.equal(classProgressRank("6B"), 16);
  assert.equal(classProgressRank("5A"), 17);
  assert.equal(classProgressRank("4F"), 18);
  assert.equal(classProgressRank("3A"), 19);
  assert.equal(classProgressRank("1F"), 27);
});

test("libellés de groupes non absorbés par un niveau", () => {
  for (const c of ["3 ps", "5 cm", "4 gr", "6 12", "60 places"]) {
    assert.equal(niveauFromClasse(c), null, c);
  }
});
