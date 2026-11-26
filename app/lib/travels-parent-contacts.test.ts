import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  buildEleveContacts,
  findParticipant,
  isTripAccompagnateur,
  resolveContactsAccess,
  sortResponsables,
  telHref,
  tripParticipantsRaw,
} from "@/app/lib/travels-parent-contacts";
import { buildParentContactsPdf } from "@/app/lib/travels-parent-contacts-pdf";

const trip = (over: Record<string, unknown> = {}, data: Record<string, unknown> = {}) =>
  ({
    id: "t1",
    status: "VALIDE",
    ownerId: "u-owner",
    ownerName: "Olivia Owner",
    data: {
      accompagnateurs: [
        { userId: "u-escort", name: "Eric Escort", source: "directory" },
        { name: "Marie Mail", email: "Marie.Mail@ecole.fr", source: "autre" },
      ],
      ...data,
    },
    ...over,
  }) as never;

const user = (id: string, roles: string[] = ["professeur"], email = `${id}@ecole.fr`, fullName = id) =>
  ({
    id,
    fullName,
    publicMetadata: { role: roles },
    primaryEmailAddress: { emailAddress: email },
  }) as never;

describe("droits sur les contacts des parents", () => {
  it("organisateur : oui", () => assert.equal(resolveContactsAccess(trip(), user("u-owner")).allowed, true));
  it("accompagnateur inscrit (id) : oui", () => assert.equal(resolveContactsAccess(trip(), user("u-escort")).allowed, true));
  it("accompagnateur inscrit (e-mail de compte, casse ignorée) : oui", () =>
    assert.equal(resolveContactsAccess(trip(), user("u-x", ["professeur"], "marie.mail@ECOLE.fr")).allowed, true));
  it("accompagnateur d'un AUTRE voyage : non", () =>
    assert.equal(resolveContactsAccess(trip({}, { accompagnateurs: [{ userId: "u-other", name: "Z", source: "directory" }] }), user("u-escort")).allowed, false));
  it("un simple homonyme (même nom, autre compte) : non", () =>
    assert.equal(resolveContactsAccess(trip(), user("u-zzz", ["professeur"], "zzz@x.fr", "Eric Escort")).allowed, false));
  it("professeur quelconque : non", () => assert.equal(resolveContactsAccess(trip(), user("u-prof")).allowed, false));
  it("direction : non", () => assert.equal(resolveContactsAccess(trip(), user("u-dir", ["direction"])).allowed, false));
  it("compta : non", () => assert.equal(resolveContactsAccess(trip(), user("u-compta", ["compta"])).allowed, false));
  it("administratif : non", () => assert.equal(resolveContactsAccess(trip(), user("u-adm", ["administratif"])).allowed, false));
  it("admin général : oui", () => assert.equal(resolveContactsAccess(trip(), user("u-admin", ["admin"])).allowed, true));
  it("sans utilisateur : non", () => assert.equal(resolveContactsAccess(trip(), null).allowed, false));
  it("voyage annulé : refusé même à l'organisateur, admin général seulement", () => {
    const t = trip({ status: "ANNULE" });
    assert.equal(resolveContactsAccess(t, user("u-owner")).allowed, false);
    assert.equal(resolveContactsAccess(t, user("u-escort")).allowed, false);
    assert.equal(resolveContactsAccess(t, user("u-admin", ["admin"])).allowed, true);
  });
  it("accompagnateur sans id ni e-mail (saisie libre) : jamais reconnu par le nom", () =>
    assert.equal(isTripAccompagnateur(trip({}, { accompagnateurs: [{ name: "Eric Escort", source: "autre" }] }), user("u-escort2", ["professeur"], "e@x.fr", "Eric Escort")), false));
});

describe("participants", () => {
  it("liste officielle prioritaire", () =>
    assert.deepEqual(tripParticipantsRaw(trip({}, { participantEleves: [{ ine: "A" }], preListeEleves: [{ ine: "B" }] })).map((p) => p.ine), ["A"]));
  it("sinon pré-liste v2", () =>
    assert.deepEqual(tripParticipantsRaw(trip({}, { preListeEleves: [{ ine: "B" }] })).map((p) => p.ine), ["B"]));
  it("aucune liste : aucun participant (jamais la classe)", () => assert.deepEqual(tripParticipantsRaw(trip()), []));
});

const resp = (o: Record<string, unknown>) => ({
  id: "r",
  prenom: "Anne",
  nom: "Dupont",
  email: null,
  telephone: null,
  autoriteParentale: false,
  contactUrgence: false,
  rang: 1,
  ...o,
});
const eleveRef = { id: "e1", nom: "Martin", prenom: "Léo", classe: "3A" };

describe("contenu des contacts", () => {
  it("autorité parentale en premier, puis rang", () => {
    const s = sortResponsables([
      { autoriteParentale: false, rang: 1, n: "a" },
      { autoriteParentale: true, rang: 3, n: "b" },
      { autoriteParentale: true, rang: 2, n: "c" },
    ]);
    assert.deepEqual(s.map((x) => x.n), ["c", "b", "a"]);
  });
  it("plusieurs foyers : groupes « Famille de… », foyer avec autorité d'abord, liens conservés", () => {
    const c = buildEleveContacts({
      eleve: eleveRef,
      fiche: {},
      foyers: [
        { foyerId: "f1", label: "Foyer", responsables: [resp({ id: "r1", prenom: "Paul", nom: "Roux", rang: 1, lien: "Beau-père", telephone: "0600000001" })] },
        {
          foyerId: "f2",
          label: "Foyer",
          responsables: [
            resp({ id: "r2", prenom: "Zoe", nom: "Martin", rang: 1, telephone: "0600000002" }),
            resp({ id: "r3", prenom: "Marc", nom: "Martin", rang: 2, autoriteParentale: true, lien: "Père", telephone: "0600000003" }),
          ],
        },
      ],
    });
    assert.equal(c.groups.length, 2);
    assert.equal(c.groups[0]!.key, "f2");
    assert.equal(c.groups[0]!.titre, "Famille de Zoe MARTIN");
    assert.deepEqual(c.groups[0]!.responsables.map((r) => r.prenom), ["Marc", "Zoe"]);
    assert.equal(c.groups[0]!.responsables[0]!.lien, "Père");
    assert.equal(c.groups[1]!.responsables[0]!.lien, "Beau-père");
  });
  it("table des liens absente : lien null, aucun plantage", () => {
    const c = buildEleveContacts({ eleve: eleveRef, fiche: {}, foyers: [{ foyerId: "f", label: "F", responsables: [resp({ telephone: "0611111111" })] }] });
    assert.equal(c.groups[0]!.responsables[0]!.lien, null);
  });
  it("secours fiche élève seulement si aucun foyer, marqué « fiche »", () => {
    const c = buildEleveContacts({
      eleve: eleveRef,
      foyers: [],
      fiche: { parent1Phone: "0601", parent2Phone: "0602", parentPhone: "0601", parent1Email: "a@x.fr", parentEmail: "a@x.fr" },
    });
    assert.equal(c.groups.length, 1);
    assert.equal(c.groups[0]!.source, "fiche");
    assert.equal(c.groups[0]!.responsables.length, 2);
    const withFoyer = buildEleveContacts({ eleve: eleveRef, fiche: { parent1Phone: "0601" }, foyers: [{ foyerId: "f", label: "F", responsables: [resp({})] }] });
    assert.equal(withFoyer.groups.every((g) => g.source === "foyer"), true);
  });
  it("tel: nettoyé", () => {
    assert.equal(telHref("06 12.34 56-78"), "tel:0612345678");
    assert.equal(telHref(""), null);
  });
});

describe("PDF fiche contacts", () => {
  it("contient l'en-tête confidentiel, le titre, l'éditeur, les participants et le pied", () => {
    const eleves = [
      buildEleveContacts({ eleve: eleveRef, fiche: {}, foyers: [{ foyerId: "f", label: "F", responsables: [resp({ prenom: "Marc", nom: "Martin", telephone: "0600000003", autoriteParentale: true, lien: "Père" })] }] }),
    ];
    const bytes = buildParentContactsPdf({ tripTitle: "Voyage a Rome", editedAt: new Date("2026-11-26T10:00:00Z"), editedBy: "Olivia Owner", eleves });
    const txt = Buffer.from(bytes).toString("latin1");
    assert.ok(txt.startsWith("%PDF"));
    assert.ok(txt.includes("CONFIDENTIEL - usage pendant le voyage uniquement"));
    assert.ok(txt.includes("Voyage a Rome"));
    assert.ok(txt.includes("Olivia Owner"));
    assert.ok(txt.includes("Document"));
    assert.ok(txt.includes("MARTIN"));
    assert.ok(txt.includes("0600000003"));
    assert.ok(!txt.includes("Eleve Inconnu"));
  });
});

describe("élève non participant", () => {
  it("refusé (null → 404)", () => {
    const list = [{ eleveId: "a" }, { eleveId: "b" }];
    assert.equal(findParticipant(list, "c"), null);
    assert.deepEqual(findParticipant(list, "b"), { eleveId: "b" });
  });
});
