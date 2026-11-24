import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  buildFamilyPagePreviewModel,
  buildFamilyPageSystemPrompt,
  buildFamilyPageUserPrompt,
  ensureFinancialDifficultySentence,
  FAMILY_PAGE_PROJECT_BANNER,
  FINANCIAL_DIFFICULTY_SENTENCE,
  stripMarkdown,
} from "@/app/lib/travels-family-page";
import { textMentionsPrice } from "@/app/lib/travels-v2";
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
    startDate: "2027-03-29",
    endDate: "2027-04-02",
    startTime: "07:30",
    endTime: "19:00",
    needsBus: true,
    coutTotal: 12000,
    costPerStudent: 300,
    transportRequest: { pickupPoint: "Parking du lycée" },
    familyPage: {
      text: "Premier paragraphe.\n\nSecond paragraphe.",
      deadline: "2027-03-10",
      documents: [
        { id: "a", label: "Aisance aquatique", kind: "avant_signature" },
        { id: "b", label: "Carte d'identité", kind: "avant_depart", dueDate: "2027-03-20" },
        { id: "c", label: "Modèle d'autorisation", kind: "facultative" },
      ],
    },
  },
} as unknown as TravelsTrip;

describe("page des familles — prompt du brouillon IA", () => {
  const system = buildFamilyPageSystemPrompt();
  it("interdit prix et document de santé, impose la phrase sur les difficultés financières", () => {
    assert.match(system, /INTERDIT : mentionner un prix/);
    assert.match(system, /INTERDIT : demander un document/);
    assert.ok(system.includes(FINANCIAL_DIFFICULTY_SENTENCE));
    assert.match(FINANCIAL_DIFFICULTY_SENTENCE, /secrétariat/);
    assert.match(FINANCIAL_DIFFICULTY_SENTENCE, /direction/);
  });
  it("n'injecte aucun montant du dossier dans la demande", () => {
    const user = buildFamilyPageUserPrompt(trip);
    assert.ok(!user.includes("12000"));
    assert.ok(!user.includes("300"));
    assert.match(user, /Biarritz/);
    assert.match(user, /Parking du lycée/);
  });
  it("ajoute la phrase financière si le modèle l'a oubliée, sans doublon sinon", () => {
    const added = ensureFinancialDifficultySentence("Texte.");
    assert.ok(added.endsWith(FINANCIAL_DIFFICULTY_SENTENCE));
    assert.equal(ensureFinancialDifficultySentence(added), added);
  });
  it("retire le Markdown", () => {
    assert.equal(stripMarkdown("# Titre\n**gras** et *italique*"), "Titre\ngras et italique");
  });
  it("détecte un prix écrit dans le texte", () => {
    assert.equal(textMentionsPrice("Le séjour coûte 350 €"), true);
    assert.equal(textMentionsPrice("Tarif : 120 euros"), true);
    assert.equal(textMentionsPrice("Rendez-vous à 7h30 devant le lycée"), false);
  });
});

describe("page des familles — aperçu", () => {
  it("bandeau PROJET tant que la page n'est pas ouverte", () => {
    const m = buildFamilyPagePreviewModel(trip);
    assert.equal(m.isProject, true);
    assert.equal(m.projectBanner, FAMILY_PAGE_PROJECT_BANNER);
  });
  it("plus de bandeau une fois ouverte", () => {
    const opened = {
      ...trip,
      data: { ...trip.data, familyPage: { ...trip.data.familyPage!, openedAt: "2027-03-01T08:00:00Z" } },
    } as TravelsTrip;
    const m = buildFamilyPagePreviewModel(opened);
    assert.equal(m.isProject, false);
    assert.equal(m.projectBanner, null);
  });
  it("reprend image, dates, RDV, programme, pièces triées et date limite", () => {
    const m = buildFamilyPagePreviewModel(trip);
    assert.equal(m.imageUrl, "https://example.org/img.jpg");
    assert.equal(m.dateLine, "Du 29/03/2027 au 02/04/2027");
    assert.ok(m.meetingLines[0].includes("07:30") && m.meetingLines[0].includes("Parking du lycée"));
    assert.equal(m.paragraphs.length, 2);
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
  it("un texte en cours de rédaction remplace le texte stocké dans l'aperçu", () => {
    const m = buildFamilyPagePreviewModel(trip, { familyPage: { text: "Autre texte" } });
    assert.deepEqual(m.paragraphs, ["Autre texte"]);
  });
});
