import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { buildFamilyPagePreviewModel, FAMILY_PAGE_PREVIEW_LABEL, FAMILY_PAGE_SAMPLE_STUDENT, listPicnicDays } from "@/app/lib/travels-family-page";
import { FAMILY_PAGE_TEMPLATE, fillFamilyTemplate } from "@/app/lib/travels-family-page-template";
import type { TravelsTrip } from "@/app/lib/travels-types";

const trip = {
  id: "t1",
  type: "COMPLEX",
  status: "EN_ATTENTE_COMPTA",
  ownerName: "M. Dupont",
  imageUrl: "https://example.org/img.jpg",
  data: {
    workflowVersion: 2,
    title: "Séjour surf",
    destination: "Biarritz",
    classes: "3A, 3B",
    etablissement: "Collège",
    startDate: "2027-03-29",
    endDate: "2027-04-02",
    coutTotal: 12000,
    costPerStudent: 300,
    parentCalendar: {
      points: [
        { id: "d", kind: "depot", date: "2027-03-29", time: "07:30", place: "Parking du lycée" },
        { id: "r", kind: "recuperation", date: "2027-04-02", time: "19:00", place: "Gare" },
      ],
    },
    familyPage: {
      deadline: "2027-03-10",
      equipment: ["Maillot de bain", "Serviette"],
      cash: { amount: 5, reason: "sandwich sur la route du retour" },
      program: "Surf le matin, visite l'après-midi.",
      documents: [
        { id: "a", label: "Aisance aquatique", kind: "avant_signature" },
        { id: "b", label: "Carte d'identité", kind: "avant_depart", dueDate: "2027-03-20" },
        { id: "c", label: "Modèle d'autorisation", kind: "facultative" },
      ],
    },
  },
} as unknown as TravelsTrip;

describe("page des familles — trame fixe", () => {
  it("la trame contient les phrases imposées", () => {
    assert.match(FAMILY_PAGE_TEMPLATE.financialHelp, /difficulté financière/);
    assert.match(FAMILY_PAGE_TEMPLATE.financialHelp, /chef d’établissement/);
    assert.match(FAMILY_PAGE_TEMPLATE.dataRetention, /lendemain du retour/);
    assert.match(FAMILY_PAGE_TEMPLATE.dataRetention, /comptabilité/);
    assert.match(FAMILY_PAGE_TEMPLATE.meals.join(" "), /sans porc/);
    assert.match(FAMILY_PAGE_TEMPLATE.meals.join(" "), /PAI/);
    assert.match(FAMILY_PAGE_TEMPLATE.cashNotIncluded, /pas comprise dans le prix/);
  });
  it("remplit les variables, « … » si inconnue", () => {
    assert.equal(fillFamilyTemplate("A {x} B {y}", { x: "1" }), "A 1 B …");
  });
  it("introduction : titre, destination, dates, classes, organisateur, date limite", () => {
    const intro = buildFamilyPagePreviewModel(trip).intro.join(" ");
    assert.match(intro, /Séjour surf/);
    assert.match(intro, /Biarritz/);
    assert.match(intro, /du 29\/03\/2027 au 02\/04\/2027/);
    assert.match(intro, /3A, 3B/);
    assert.match(intro, /M\. Dupont/);
    assert.match(intro, /10\/03\/2027/);
  });
});

describe("page des familles — aperçu", () => {
  it("simple étiquette « Aperçu », pas de bandeau projet", () => {
    const m = buildFamilyPagePreviewModel(trip);
    assert.equal(m.previewLabel, FAMILY_PAGE_PREVIEW_LABEL);
    assert.equal(FAMILY_PAGE_PREVIEW_LABEL, "Aperçu");
    assert.equal(m.opened, false);
  });
  it("plus d'étiquette une fois ouverte", () => {
    const opened = {
      ...trip,
      data: { ...trip.data, familyPage: { ...trip.data.familyPage!, openedAt: "2027-03-01T08:00:00Z" } },
    } as TravelsTrip;
    const m = buildFamilyPagePreviewModel(opened);
    assert.equal(m.opened, true);
    assert.equal(m.previewLabel, null);
  });
  it("reprend image, rendez-vous du calendrier parents, programme, équipement, liquide, pièces triées, date limite", () => {
    const m = buildFamilyPagePreviewModel(trip);
    assert.equal(m.imageUrl, "https://example.org/img.jpg");
    assert.equal(m.outbound?.place, "Parking du lycée");
    assert.equal(m.outbound?.time, "07:30");
    assert.equal(m.inbound?.place, "Gare");
    assert.equal(m.program, "Surf le matin, visite l'après-midi.");
    assert.deepEqual(m.equipment, ["Maillot de bain", "Serviette"]);
    assert.deepEqual(m.cash, { amount: 5, reason: "sandwich sur la route du retour" });
    assert.deepEqual(m.documents.map((d) => d.id), ["a", "b", "c"]);
    assert.equal(m.deadline, "2027-03-10");
  });
  it("le prix vient uniquement du bloc prix de la compta, jamais du budget interne", () => {
    assert.equal(buildFamilyPagePreviewModel(trip).price, null);
    const withPrice = {
      ...trip,
      data: {
        ...trip.data,
        priceBlock: { pricePerStudent: 180, installments: [{ id: "x", dueDate: "2027-01-15", amount: 180 }], paymentText: "Chèque" },
      },
    } as TravelsTrip;
    const m = buildFamilyPagePreviewModel(withPrice);
    assert.equal(m.price?.pricePerStudent, 180);
    assert.equal(m.price?.installments.length, 1);
  });
  it("liquide sans motif ou programme vide : non affichés", () => {
    const t = { ...trip, data: { ...trip.data, familyPage: { documents: [], cash: { amount: 5, reason: "" } } } } as unknown as TravelsTrip;
    const m = buildFamilyPagePreviewModel(t);
    assert.equal(m.cash, null);
    assert.equal(m.program, "");
  });
  it("un brouillon en cours remplace les valeurs stockées dans l'aperçu", () => {
    const m = buildFamilyPagePreviewModel(trip, { familyPage: { program: "Autre programme" } });
    assert.equal(m.program, "Autre programme");
  });
});

describe("page des familles — valeurs en attente et paniers", () => {
  it("dossier complet sauf le prix : seul le montant est en attente", () => {
    const m = buildFamilyPagePreviewModel(trip);
    assert.deepEqual(m.pending.map((p) => p.key), ["price"]);
    assert.equal(m.pending[0].label, "Montant en attente de la comptabilité");
  });
  it("dossier vide : rendez-vous, montant et date limite signalés", () => {
    const empty = { ...trip, data: { workflowVersion: 2, title: "X", familyPage: { documents: [] } } } as unknown as TravelsTrip;
    const m = buildFamilyPagePreviewModel(empty);
    assert.deepEqual(m.pending.map((p) => p.key).sort(), ["deadline", "inbound", "outbound", "price"]);
    assert.ok(m.pending.some((p) => p.label === "Date limite à fixer"));
    assert.ok(m.pending.some((p) => p.label === "Rendez-vous de départ à compléter"));
  });
  it("rendez-vous sans heure : encore en attente", () => {
    const t = {
      ...trip,
      data: { ...trip.data, parentCalendar: { points: [{ id: "d", kind: "depot", date: "2027-03-29", time: "", place: "Parking" }] } },
    } as unknown as TravelsTrip;
    assert.ok(buildFamilyPagePreviewModel(t).pending.some((p) => p.key === "outbound"));
  });
  it("prix saisi par la compta et date limite : plus en attente", () => {
    const t = {
      ...trip,
      data: { ...trip.data, priceBlock: { pricePerStudent: 0, installments: [], paymentText: "" } },
    } as unknown as TravelsTrip;
    assert.deepEqual(buildFamilyPagePreviewModel(t).pending, []);
  });
  it("jours de pique-nique : dates réelles cochées, triées", () => {
    const days = listPicnicDays({ active: true, daysSelection: { "2027-03-31": true, "2027-03-29": true, "2027-03-30": false, lundi: true } });
    assert.deepEqual(days.map((d) => d.key), ["2027-03-29", "2027-03-31"]);
    assert.match(days[0].label, /lun\. 29\/03/);
  });
  it("jours de pique-nique : structure historique et cuisine inactive", () => {
    assert.deepEqual(
      listPicnicDays({ active: true, daysSelection: { vendredi: true, lundi: true, mardi: false } }).map((d) => d.label),
      ["Lundi", "Vendredi"],
    );
    assert.deepEqual(listPicnicDays({ active: false, daysSelection: { lundi: true } }), []);
    assert.deepEqual(listPicnicDays(undefined), []);
  });
  it("l'aperçu transporte les jours de pique-nique du dossier et un élève fictif neutre", () => {
    const t = {
      ...trip,
      data: { ...trip.data, piqueNiqueDetails: { active: true, daysSelection: { "2027-03-30": true } } },
    } as unknown as TravelsTrip;
    assert.equal(buildFamilyPagePreviewModel(t).picnicDays.length, 1);
    assert.equal(FAMILY_PAGE_SAMPLE_STUDENT, "Prénom NOM");
  });
});
