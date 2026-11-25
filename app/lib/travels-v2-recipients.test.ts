import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  mergeRecipients,
  pickFoyerResponsables,
  studentsWithoutRecipient,
  validateExtraRecipient,
  type FoyerResponsableInput,
} from "@/app/lib/travels-v2-recipients";

const r = (o: Partial<FoyerResponsableInput> & { email: string | null }): FoyerResponsableInput => ({
  foyerId: "f1",
  autoriteParentale: false,
  nom: "Dupont",
  prenom: "Anne",
  rang: 1,
  ...o,
});

describe("pickFoyerResponsables", () => {
  it("ne garde que les responsables avec autorité parentale quand certains sont cochés", () => {
    const out = pickFoyerResponsables([
      r({ email: "a@x.fr", autoriteParentale: true }),
      r({ email: "b@x.fr", autoriteParentale: false, rang: 2 }),
    ]);
    assert.deepEqual(out.map((x) => x.email), ["a@x.fr"]);
  });
  it("sans aucun drapeau : prend tous les responsables du foyer ayant un e-mail", () => {
    const out = pickFoyerResponsables([r({ email: "a@x.fr" }), r({ email: "b@x.fr", rang: 2 }), r({ email: null, rang: 3 })]);
    assert.deepEqual(out.map((x) => x.email), ["a@x.fr", "b@x.fr"]);
  });
  it("drapeau coché seulement sur un responsable sans e-mail : bascule sur tous ceux qui ont un e-mail", () => {
    const out = pickFoyerResponsables([r({ email: "", autoriteParentale: true }), r({ email: "b@x.fr", rang: 2 })]);
    assert.deepEqual(out.map((x) => x.email), ["b@x.fr"]);
  });
  it("applique la règle foyer par foyer", () => {
    const out = pickFoyerResponsables([
      r({ foyerId: "f1", email: "a@x.fr", autoriteParentale: true }),
      r({ foyerId: "f1", email: "b@x.fr", rang: 2 }),
      r({ foyerId: "f2", email: "c@x.fr" }),
      r({ foyerId: "f2", email: "d@x.fr", rang: 2 }),
    ]);
    assert.deepEqual(out.map((x) => x.email).sort(), ["a@x.fr", "c@x.fr", "d@x.fr"]);
  });
  it("ignore les e-mails invalides", () => {
    assert.deepEqual(pickFoyerResponsables([r({ email: "pas-un-mail", autoriteParentale: true })]), []);
  });
});

describe("mergeRecipients", () => {
  it("fusionne fiche + foyer + adresse ajoutée, dédoublonne sans tenir compte de la casse", () => {
    const out = mergeRecipients({
      ficheEmails: ["Parent@Mail.fr", "autre@mail.fr"],
      foyerResponsables: [r({ email: "PARENT@mail.fr", autoriteParentale: true }), r({ email: "tiers@mail.fr", autoriteParentale: true, rang: 2, nom: "Martin", prenom: "Luc" })],
      extras: [{ email: " Tiers@Mail.fr ", name: "Autre nom" }, { email: "ajout@mail.fr", name: "Tante" }],
    });
    assert.deepEqual(out.map((x) => x.email), ["parent@mail.fr", "autre@mail.fr", "tiers@mail.fr", "ajout@mail.fr"]);
    const parent = out.find((x) => x.email === "parent@mail.fr");
    assert.equal(parent?.source, "fiche");
    assert.equal(parent?.name, "Anne Dupont"); // nom repris du foyer
    const tiers = out.find((x) => x.email === "tiers@mail.fr");
    assert.equal(tiers?.source, "foyer");
    assert.equal(tiers?.name, "Luc Martin");
    assert.equal(out.find((x) => x.email === "ajout@mail.fr")?.source, "ajoutee");
  });
  it("écarte les e-mails invalides ou vides", () => {
    const out = mergeRecipients({ ficheEmails: ["", "abc", "a@b"], extras: [{ email: "x y@z.fr" }] });
    assert.deepEqual(out, []);
  });
  it("une adresse ajoutée déjà connue en foyer garde l'origine foyer", () => {
    const out = mergeRecipients({ foyerResponsables: [r({ email: "a@x.fr" })], extras: [{ email: "a@x.fr" }] });
    assert.equal(out.length, 1);
    assert.equal(out[0].source, "foyer");
  });
  it("sans aucune source : liste vide", () => {
    assert.deepEqual(mergeRecipients({}), []);
  });
});

describe("studentsWithoutRecipient", () => {
  it("retourne les élèves sans adresse (INE dédoublonnés, casse ignorée)", () => {
    const by = new Map([["AAA", mergeRecipients({ ficheEmails: ["a@x.fr"] })], ["BBB", []]]);
    const out = studentsWithoutRecipient([{ ine: "aaa" }, { ine: "bbb" }, { ine: "BBB" }, { ine: "ccc" }, { ine: "" }], by);
    assert.deepEqual(out.map((s) => s.ine), ["bbb", "ccc"]);
  });
});

describe("validateExtraRecipient", () => {
  it("normalise et valide", () => {
    const v = validateExtraRecipient("  Jean@Mail.FR ", "  Jean   Dupont ");
    assert.deepEqual(v, { ok: true, email: "jean@mail.fr", name: "Jean Dupont" });
  });
  it("refuse une adresse invalide ou un nom trop long", () => {
    assert.equal(validateExtraRecipient("nope", "").ok, false);
    assert.equal(validateExtraRecipient("a@b.fr", "x".repeat(101)).ok, false);
  });
  it("nom facultatif", () => {
    assert.deepEqual(validateExtraRecipient("a@b.fr", undefined), { ok: true, email: "a@b.fr" });
  });
});
