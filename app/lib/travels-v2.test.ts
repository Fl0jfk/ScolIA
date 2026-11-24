import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  addDaysIso,
  canOpenRegistrations,
  checkWorkflowV2Update,
  computeRetroplanning,
  diffDaysIso,
  evaluateFamilyPageTextChange,
  evaluatePriceBlockChange,
  familyPageReadiness,
  HEALTH_DOC_FORBIDDEN_MESSAGE,
  isHealthRelatedLabel,
  isWorkflowV2,
  listTripDays,
  mirrorPiqueNiqueDatesToWeekdays,
  shouldCreateTripAsV2,
  shouldSendTransportOnPedagogyValidation,
  statusAfterPedagogyV2,
  validateDeadlineForOpening,
  validateFamilyDocuments,
  validatePriceBlock,
} from "@/app/lib/travels-v2";
import { travelsListPipeline } from "@/app/lib/travels-trip-helpers";
import { getTripNextGuidance } from "@/app/lib/travels-next-guidance";
import { buildTripWorkflowAudit } from "@/app/lib/travels-workflow-audit";
import { TRAVELS_STATUS_LABELS, type TravelsTrip } from "@/app/lib/travels-types";

const PRICE = {
  pricePerStudent: 120,
  installments: [
    { id: "a", dueDate: "2027-01-10", amount: 60 },
    { id: "b", dueDate: "2027-02-10", amount: 60 },
  ],
  paymentText: "Virement ou chèque",
};

const compta = { isCompta: true, isDirectionOfEtab: false, isGlobalAdmin: false };
const direction = { isCompta: false, isDirectionOfEtab: true, isGlobalAdmin: false };
const admin = { isCompta: false, isDirectionOfEtab: false, isGlobalAdmin: true };
const prof = { isCompta: false, isDirectionOfEtab: false, isGlobalAdmin: false };

describe("travels v2 — version", () => {
  it("détecte la v2 uniquement avec workflowVersion === 2", () => {
    assert.equal(isWorkflowV2({ data: { workflowVersion: 2 } }), true);
    assert.equal(isWorkflowV2({ data: {} }), false);
    assert.equal(isWorkflowV2({ data: { workflowVersion: 1 } }), false);
    assert.equal(isWorkflowV2(null), false);
  });

  it("les séries récurrentes restent en v1, même drapeau actif", () => {
    assert.equal(shouldCreateTripAsV2({ flagEnabled: true, isRecurrenceSeries: false }), true);
    assert.equal(shouldCreateTripAsV2({ flagEnabled: true, isRecurrenceSeries: true }), false);
    assert.equal(shouldCreateTripAsV2({ flagEnabled: false }), false);
    assert.equal(shouldCreateTripAsV2({ flagEnabled: undefined }), false);
  });
});

describe("travels v2 — documents de santé interdits", () => {
  const refused = [
    "Fiche sanitaire",
    "FICHE SANITAIRE de liaison",
    "Certificat médical",
    "certificat medical d'aptitude",
    "Ordonnance",
    "Carnet de vaccination",
    "Vaccins à jour",
    "PAI",
    "Copie du P.A.I.",
    "Projet d’accueil individualisé",
    "Allergies alimentaires",
    "Traitement en cours",
    "Informations santé",
    "Dossier médical",
    "Médicaments",
  ];
  for (const label of refused) {
    it(`refuse « ${label} »`, () => {
      assert.equal(isHealthRelatedLabel(label), true);
      const res = validateFamilyDocuments([{ id: "1", label, kind: "avant_signature" }]);
      assert.equal(res.ok, false);
      if (!res.ok) assert.equal(res.error, HEALTH_DOC_FORBIDDEN_MESSAGE);
    });
  }

  const accepted = [
    "Attestation d'aisance aquatique",
    "Autorisation de sortie du territoire",
    "Photocopie de la carte d'identité",
    "Paiement de la première échéance",
    "Pain et fromage",
    "Assurance responsabilité civile",
  ];
  for (const label of accepted) {
    it(`accepte « ${label} »`, () => {
      assert.equal(isHealthRelatedLabel(label), false);
      assert.deepEqual(validateFamilyDocuments([{ id: "1", label, kind: "facultative" }]), { ok: true });
    });
  }

  it("le texte d'interdiction mentionne Charlemagne et École Directe", () => {
    assert.match(HEALTH_DOC_FORBIDDEN_MESSAGE, /Charlemagne et École Directe/);
  });

  it("contrôle aussi le nom du modèle vierge", () => {
    const res = validateFamilyDocuments([
      { id: "1", label: "Autorisation", kind: "facultative", template: { name: "Fiche sanitaire.pdf", url: "x" } },
    ]);
    assert.equal(res.ok, false);
  });

  it("exige une date pour une pièce « avant départ »", () => {
    assert.equal(validateFamilyDocuments([{ id: "1", label: "Carte d'identité", kind: "avant_depart" }]).ok, false);
    assert.equal(
      validateFamilyDocuments([{ id: "1", label: "Carte d'identité", kind: "avant_depart", dueDate: "2027-03-01" }], {
        departureIso: "2027-03-10",
      }).ok,
      true,
    );
    assert.equal(
      validateFamilyDocuments([{ id: "1", label: "Carte d'identité", kind: "avant_depart", dueDate: "2027-04-01" }], {
        departureIso: "2027-03-10",
      }).ok,
      false,
    );
  });

  it("refuse un intitulé vide ou un type inconnu", () => {
    assert.equal(validateFamilyDocuments([{ id: "1", label: "  ", kind: "facultative" }]).ok, false);
    assert.equal(validateFamilyDocuments([{ id: "1", label: "X", kind: "autre" }]).ok, false);
  });
});

describe("travels v2 — rétroplanning", () => {
  it("clôture à J-14 et validation finale à J-21", () => {
    const r = computeRetroplanning("2027-03-31", "2027-01-04");
    assert.ok(r);
    assert.equal(r.latestDeadlineIso, "2027-03-17");
    assert.equal(r.latestFinalValidationIso, "2027-03-10");
    assert.equal(r.tooClose, false);
    assert.equal(r.warning, null);
  });

  it("avertit si le départ est trop proche", () => {
    const r = computeRetroplanning("2027-03-31", "2027-03-15");
    assert.ok(r);
    assert.equal(r.tooClose, true);
    assert.match(r.warning || "", /trop proche/);
  });

  it("limite exacte J-21 acceptée, J-20 trop proche", () => {
    assert.equal(computeRetroplanning("2027-03-31", "2027-03-10")?.tooClose, false);
    assert.equal(computeRetroplanning("2027-03-31", "2027-03-11")?.tooClose, true);
  });

  it("date invalide → null", () => {
    assert.equal(computeRetroplanning("", "2027-03-10"), null);
    assert.equal(computeRetroplanning("2027-13-40", "2027-03-10"), null);
  });

  it("calcul de dates", () => {
    assert.equal(addDaysIso("2027-03-01", -14), "2027-02-15");
    assert.equal(diffDaysIso("2027-03-01", "2027-03-31"), 30);
  });
});

describe("travels v2 — date limite à la validation finale", () => {
  const base = { departureIso: "2027-03-31", todayIso: "2027-02-01" };
  it("refuse une date absente", () => {
    assert.equal(validateDeadlineForOpening({ ...base, deadline: "" }).ok, false);
    assert.equal(validateDeadlineForOpening({ ...base, deadline: undefined }).ok, false);
  });
  it("refuse moins de 7 jours après aujourd'hui (« repoussez »)", () => {
    const res = validateDeadlineForOpening({ ...base, deadline: "2027-02-07" });
    assert.equal(res.ok, false);
    if (!res.ok) assert.match(res.error, /repoussez la date limite/);
  });
  it("accepte exactement aujourd'hui + 7", () => {
    assert.equal(validateDeadlineForOpening({ ...base, deadline: "2027-02-08" }).ok, true);
  });
  it("refuse après J-14", () => {
    const res = validateDeadlineForOpening({ ...base, deadline: "2027-03-18" });
    assert.equal(res.ok, false);
    if (!res.ok) assert.match(res.error, /Avancez la date limite/);
  });
  it("accepte exactement J-14", () => {
    assert.equal(validateDeadlineForOpening({ ...base, deadline: "2027-03-17" }).ok, true);
  });
});

describe("travels v2 — bloc prix", () => {
  it("accepte un bloc complet", () => {
    assert.deepEqual(validatePriceBlock(PRICE), { ok: true });
  });
  it("accepte 0 € sans texte de paiement", () => {
    assert.deepEqual(validatePriceBlock({ pricePerStudent: 0, installments: [], paymentText: "" }), { ok: true });
  });
  it("refuse l'absence de bloc, de prix ou de modalités", () => {
    assert.equal(validatePriceBlock(undefined).ok, false);
    assert.equal(validatePriceBlock({ installments: [], paymentText: "x" }).ok, false);
    assert.equal(validatePriceBlock({ pricePerStudent: 100, installments: [], paymentText: "" }).ok, false);
  });
  it("refuse des échéances qui ne totalisent pas le prix", () => {
    assert.equal(
      validatePriceBlock({ ...PRICE, installments: [{ id: "a", dueDate: "2027-01-10", amount: 50 }] }).ok,
      false,
    );
  });
});

describe("travels v2 — règles serveur pures", () => {
  it("bloc prix : compta seule", () => {
    assert.equal(evaluatePriceBlockChange({ existing: undefined, incoming: PRICE, isCompta: true }), "ok");
    assert.equal(evaluatePriceBlockChange({ existing: undefined, incoming: PRICE, isCompta: false }), "forbidden");
    assert.equal(evaluatePriceBlockChange({ existing: PRICE, incoming: { ...PRICE, pricePerStudent: 99 }, isCompta: false }), "forbidden");
    assert.equal(evaluatePriceBlockChange({ existing: PRICE, incoming: { ...PRICE }, isCompta: false }), "ok");
    assert.equal(evaluatePriceBlockChange({ existing: PRICE, incoming: undefined, isCompta: false }), "restore");
    assert.equal(evaluatePriceBlockChange({ existing: undefined, incoming: undefined, isCompta: false }), "ok");
  });

  it("ouverture : direction ou admin général uniquement", () => {
    assert.equal(canOpenRegistrations(direction), true);
    assert.equal(canOpenRegistrations(admin), true);
    assert.equal(canOpenRegistrations(compta), false);
    assert.equal(canOpenRegistrations(prof), false);
  });

  it("texte verrouillé après ouverture", () => {
    assert.equal(evaluateFamilyPageTextChange({ existing: { text: "a" }, incoming: { text: "b" } }), "ok");
    assert.equal(
      evaluateFamilyPageTextChange({ existing: { text: "a", openedAt: "2027-01-01T00:00:00Z" }, incoming: { text: "b" } }),
      "forbidden",
    );
    assert.equal(
      evaluateFamilyPageTextChange({ existing: { text: "a", openedAt: "2027-01-01T00:00:00Z" }, incoming: { text: "a" } }),
      "ok",
    );
  });

  it("transport : envoi automatique une seule fois au passage pédago → logistique", () => {
    const base = { isV2: true, previousStatus: "EN_ATTENTE_DIR_INITIAL", newStatus: "PROF_LOGISTICS", tripType: "COMPLEX", needsBus: true };
    assert.equal(shouldSendTransportOnPedagogyValidation(base), true);
    assert.equal(shouldSendTransportOnPedagogyValidation({ ...base, transportRequestSentAt: "2027-01-01" }), false);
    assert.equal(shouldSendTransportOnPedagogyValidation({ ...base, isV2: false }), false);
    assert.equal(shouldSendTransportOnPedagogyValidation({ ...base, needsBus: false }), false);
    assert.equal(shouldSendTransportOnPedagogyValidation({ ...base, previousStatus: "BESOIN_MODIFICATION" }), false);
  });

  it("sans bus, la pédagogie mène directement aux finances", () => {
    assert.equal(statusAfterPedagogyV2({ tripType: "COMPLEX", needsBus: false }), "EN_ATTENTE_COMPTA");
    assert.equal(statusAfterPedagogyV2({ tripType: "COMPLEX", needsBus: true }), "PROF_LOGISTICS");
    assert.equal(statusAfterPedagogyV2({ tripType: "SIMPLE" }), "EN_ATTENTE_COMPTA");
  });
});

describe("travels v2 — checkWorkflowV2Update", () => {
  const dep = "2027-03-31";
  const today = "2027-02-01";
  const existingBase = {
    workflowVersion: 2,
    startDate: dep,
    familyPage: { text: "Bonjour", deadline: "2027-02-20", documents: [] },
    priceBlock: PRICE,
  };

  it("ne contrôle rien pour un dossier v1", () => {
    const res = checkWorkflowV2Update({
      existingData: { startDate: dep },
      incomingData: { startDate: dep, priceBlock: PRICE },
      previousStatus: "EN_ATTENTE_COMPTA",
      newStatus: "INSCRIPTIONS_OUVERTES",
      actor: prof,
      todayIso: today,
    });
    assert.deepEqual(res, { ok: true, restorePriceBlock: false, stampOpenedAt: false });
  });

  it("refuse la modification du bloc prix par un non-compta", () => {
    const res = checkWorkflowV2Update({
      existingData: { ...existingBase },
      incomingData: { ...existingBase, priceBlock: { ...PRICE, pricePerStudent: 5 } },
      previousStatus: "EN_ATTENTE_COMPTA",
      newStatus: "EN_ATTENTE_COMPTA",
      actor: prof,
      todayIso: today,
    });
    assert.equal(res.ok, false);
    if (!res.ok) assert.equal(res.status, 403);
  });

  it("restaure le bloc prix quand une copie périmée n'en contient pas", () => {
    const res = checkWorkflowV2Update({
      existingData: { ...existingBase },
      incomingData: { ...existingBase, priceBlock: undefined },
      previousStatus: "EN_ATTENTE_COMPTA",
      newStatus: "EN_ATTENTE_COMPTA",
      actor: prof,
      todayIso: today,
    });
    assert.deepEqual(res, { ok: true, restorePriceBlock: true, stampOpenedAt: false });
  });

  it("la validation finances exige le bloc prix", () => {
    const res = checkWorkflowV2Update({
      existingData: { ...existingBase, priceBlock: undefined },
      incomingData: { ...existingBase, priceBlock: undefined },
      previousStatus: "EN_ATTENTE_COMPTA",
      newStatus: "EN_ATTENTE_DIR_FINAL",
      actor: compta,
      todayIso: today,
    });
    assert.equal(res.ok, false);
    const ok = checkWorkflowV2Update({
      existingData: { ...existingBase, priceBlock: undefined },
      incomingData: { ...existingBase },
      previousStatus: "EN_ATTENTE_COMPTA",
      newStatus: "EN_ATTENTE_DIR_FINAL",
      actor: compta,
      todayIso: today,
    });
    assert.equal(ok.ok, true);
  });

  it("ouverture refusée à un non-direction, acceptée à la direction ou à l'admin", () => {
    const common = {
      existingData: { ...existingBase },
      incomingData: { ...existingBase },
      previousStatus: "EN_ATTENTE_DIR_FINAL",
      newStatus: "INSCRIPTIONS_OUVERTES",
      todayIso: today,
    };
    for (const a of [prof, compta]) {
      const res = checkWorkflowV2Update({ ...common, actor: a });
      assert.equal(res.ok, false);
      if (!res.ok) assert.equal(res.status, 403);
    }
    for (const a of [direction, admin]) {
      assert.deepEqual(checkWorkflowV2Update({ ...common, actor: a }), {
        ok: true,
        restorePriceBlock: false,
        stampOpenedAt: true,
      });
    }
  });

  it("ouverture refusée sans date limite valide", () => {
    const res = checkWorkflowV2Update({
      existingData: { ...existingBase },
      incomingData: { ...existingBase, familyPage: { ...existingBase.familyPage, deadline: "2027-02-03" } },
      previousStatus: "EN_ATTENTE_DIR_FINAL",
      newStatus: "INSCRIPTIONS_OUVERTES",
      actor: direction,
      todayIso: today,
    });
    assert.equal(res.ok, false);
    if (!res.ok) assert.match(res.error, /repoussez/);
  });

  it("refuse toute modification du texte après ouverture", () => {
    const opened = { ...existingBase, familyPage: { ...existingBase.familyPage, openedAt: "2027-02-01T10:00:00Z" } };
    const res = checkWorkflowV2Update({
      existingData: opened,
      incomingData: { ...opened, familyPage: { ...opened.familyPage, text: "Autre" } },
      previousStatus: "INSCRIPTIONS_OUVERTES",
      newStatus: "INSCRIPTIONS_OUVERTES",
      actor: direction,
      todayIso: today,
    });
    assert.equal(res.ok, false);
  });

  it("refuse une pièce de santé côté serveur", () => {
    const res = checkWorkflowV2Update({
      existingData: { ...existingBase },
      incomingData: {
        ...existingBase,
        familyPage: { ...existingBase.familyPage, documents: [{ id: "1", label: "Fiche sanitaire", kind: "avant_signature" }] },
      },
      previousStatus: "PROF_LOGISTICS",
      newStatus: "PROF_LOGISTICS",
      actor: prof,
      todayIso: today,
    });
    assert.equal(res.ok, false);
    if (!res.ok) assert.equal(res.error, HEALTH_DOC_FORBIDDEN_MESSAGE);
  });

  it("empêche de changer la version du circuit", () => {
    const res = checkWorkflowV2Update({
      existingData: { ...existingBase },
      incomingData: { ...existingBase, workflowVersion: undefined },
      previousStatus: "PROF_LOGISTICS",
      newStatus: "PROF_LOGISTICS",
      actor: prof,
      todayIso: today,
    });
    assert.equal(res.ok, false);
  });
});

describe("travels v2 — pique-nique sur les vraies dates", () => {
  it("liste les jours réels du séjour", () => {
    assert.deepEqual(listTripDays("2027-03-29", "2027-04-01"), ["2027-03-29", "2027-03-30", "2027-03-31", "2027-04-01"]);
    assert.deepEqual(listTripDays("2027-03-29", ""), ["2027-03-29"]);
    assert.deepEqual(listTripDays("", "2027-03-29"), []);
  });

  it("recopie les dates en semaine sur lundi…vendredi sans collision", () => {
    const out = mirrorPiqueNiqueDatesToWeekdays({
      active: true,
      daysSelection: { "2027-03-29": true, "2027-03-30": true, "2027-04-03": true, "2027-04-05": true },
      orders: { "2027-03-29": { picnicTotal: "10" }, "2027-03-30": { picnicTotal: "12" } },
    });
    const sel = out.daysSelection as Record<string, boolean>;
    assert.equal(sel.lundi, true); // 29/03/2027 est un lundi
    assert.equal(sel.mardi, true);
    assert.equal(sel.samedi, undefined);
    assert.deepEqual((out.orders as Record<string, unknown>).lundi, { picnicTotal: "10" });
  });
});

function v2Trip(partial: Partial<TravelsTrip> & { data?: TravelsTrip["data"] }): TravelsTrip {
  return {
    id: "t1",
    type: "COMPLEX",
    status: "EN_ATTENTE_DIR_INITIAL",
    ...partial,
    data: { workflowVersion: 2, title: "Surf", destination: "Biarritz", startDate: "2027-03-29", endDate: "2027-04-02", ...(partial.data || {}) },
  } as TravelsTrip;
}

describe("travels v2 — frise, guidage, audit", () => {
  it("statut INSCRIPTIONS_OUVERTES libellé", () => {
    assert.equal(TRAVELS_STATUS_LABELS.INSCRIPTIONS_OUVERTES, "Inscriptions des familles");
  });

  it("frise v2 avec bus : page familles en parallèle du transport puis inscriptions", () => {
    const p = travelsListPipeline(v2Trip({ data: { needsBus: true } }));
    assert.ok(p.steps.some((s) => s.id === "transport" && /Page/.test(s.label)));
    assert.ok(p.steps.some((s) => s.id === "inscriptions"));
    const open = travelsListPipeline(v2Trip({ status: "INSCRIPTIONS_OUVERTES", data: { needsBus: true } }));
    assert.equal(open.steps[open.currentIndex].id, "inscriptions");
    assert.equal(open.currentLabel, "Inscriptions des familles");
  });

  it("frise v2 sans bus : étape page familles distincte", () => {
    const p = travelsListPipeline(v2Trip({ data: { needsBus: false } }));
    assert.ok(p.steps.some((s) => s.id === "famille"));
  });

  it("la frise v1 est inchangée", () => {
    const p = travelsListPipeline({ ...v2Trip({ data: { needsBus: true } }), data: { needsBus: true } } as TravelsTrip);
    assert.deepEqual(
      p.steps.map((s) => s.id),
      ["pedago", "transport", "finances", "direction", "eleves"],
    );
  });

  it("guidage v2 : compta doit saisir le bloc prix", () => {
    const g = getTripNextGuidance(v2Trip({ status: "EN_ATTENTE_COMPTA" }), { isOwner: false, canSign: false, isCompta: true });
    assert.ok(g);
    assert.match(g.what + g.steps.join(" "), /bloc prix/i);
    assert.equal(g.youMustAct, true);
  });

  it("guidage v2 : validation finale = ouverture des inscriptions", () => {
    const g = getTripNextGuidance(v2Trip({ status: "EN_ATTENTE_DIR_FINAL" }), { isOwner: false, canSign: true, isCompta: false });
    assert.ok(g);
    assert.match(g.steps.join(" "), /date limite/i);
  });

  it("guidage v2 : page familles en parallèle du transport", () => {
    const g = getTripNextGuidance(v2Trip({ status: "PROF_LOGISTICS", data: { needsBus: true } }), {
      isOwner: true,
      canSign: false,
      isCompta: false,
    });
    assert.ok(g);
    assert.match(g.steps.join(" "), /Page familles/);
  });

  it("guidage v1 inchangé", () => {
    const trip = { id: "t", type: "COMPLEX", status: "EN_ATTENTE_DIR_FINAL", data: { needsBus: true } } as TravelsTrip;
    const g = getTripNextGuidance(trip, { isOwner: false, canSign: true, isCompta: false });
    assert.ok(g);
    assert.doesNotMatch(g.steps.join(" "), /date limite/i);
  });

  it("audit v2 : parcours attendu et manques", () => {
    const a = buildTripWorkflowAudit(v2Trip({ status: "EN_ATTENTE_COMPTA" }));
    assert.match(a.expectedPath.join(" "), /INSCRIPTIONS_OUVERTES/);
    assert.ok(a.missingForCurrentStep.some((m) => /bloc prix/i.test(m)));
  });

  it("aperçu direction : points bloquants", () => {
    const r = familyPageReadiness(v2Trip({ data: { familyPage: { text: "", documents: [] } } }).data, "2027-02-01");
    assert.equal(r.ready, false);
    assert.ok(r.problems.length >= 3);
  });
});
