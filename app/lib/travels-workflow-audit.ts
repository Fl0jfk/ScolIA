import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import { TRAVELS_STATUS_LABELS, type TravelsTrip } from "@/app/lib/travels-types";

export type TripWorkflowAudit = {
  tripId: string;
  title: string;
  type: "SIMPLE" | "COMPLEX" | string;
  status: string;
  statusLabel: string;
  withBusLogistics: boolean;
  /** Enchaînement nominal pour ce type de dossier. */
  expectedPath: string[];
  /** Qui est attendu maintenant (rôles). */
  expectedActorsNow: string[];
  /** Ce qui est déjà rempli / présent. */
  filled: string[];
  /** Ce qui manque pour l’étape courante (ou pour avancer). */
  missingForCurrentStep: string[];
  /** Freins concrets détectés dans le JSON. */
  blockers: string[];
  /** Conseils contextuels (attendre devis, choisir, compléter prix…). */
  advice: string[];
  devis: {
    count: number;
    providers: string[];
    selectedProvider: string | null;
    signed: boolean;
    pendingAmended: boolean;
    bypassed: boolean;
  };
  finances: {
    coutTotalPrevu: number | null;
    costPerStudent: number | null;
    finalTotalCost: number | null;
    comptaSheetPresent: boolean;
    budgetValidated: boolean;
    depensesTotal: number | null;
    prixParEleveAnnonce: number | null;
    prixParEleveCalcule: number | null;
  };
  effectif: {
    nbEleves: number | null;
    nbAccompagnateurs: number | null;
    listeElevesCount: number;
    listeElevesConfirmed: boolean;
  };
  /** JSON compact du projet (sans URLs lourdes) pour l’IA. */
  projectSnapshot: Record<string, unknown>;
};

function num(v: unknown): number | null {
  if (v == null || v === "") return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

function statusLabel(status: string): string {
  return TRAVELS_STATUS_LABELS[status] || status;
}

function expectedPathFor(trip: TravelsTrip, withBus: boolean): string[] {
  if (trip.type === "SIMPLE") {
    return [
      "EN_ATTENTE_DIR_INITIAL (pédagogie)",
      "EN_ATTENTE_COMPTA (finances)",
      "EN_ATTENTE_DIR_FINAL (finale)",
      "VALIDE",
    ];
  }
  if (!withBus) {
    return [
      "EN_ATTENTE_DIR_INITIAL (pédagogie)",
      "PROF_LOGISTICS (sans bus → passer finances)",
      "EN_ATTENTE_COMPTA (finances)",
      "EN_ATTENTE_DIR_FINAL (finale)",
      "VALIDE",
    ];
  }
  return [
    "EN_ATTENTE_DIR_INITIAL (pédagogie)",
    "PROF_LOGISTICS (choix devis — créateur « Choisir » ou direction « Choisir et signer »)",
    "EN_ATTENTE_BUS_SIGNATURE (signature direction si seul choix créateur)",
    "EN_ATTENTE_COMPTA (finances)",
    "EN_ATTENTE_DIR_FINAL (finale)",
    "VALIDE",
  ];
}

function stripHeavy(data: TravelsTrip["data"]): Record<string, unknown> {
  const {
    attachments: _a,
    transportEmailMessages: _m,
    parentComLogs: _p,
    documentScans: _d,
    ocrDebugLog: _o,
    ...rest
  } = data || {};
  const sheet = data?.comptaSheet;
  const comptaLite = sheet
    ? {
        nbEleves: sheet.nbEleves,
        depensesTotal: sheet.depensesTotal,
        montantCibleFacturation: sheet.montantCibleFacturation,
        prixParEleveAnnonce: sheet.prixParEleveAnnonce,
        prixParEleveAvecSubventions: sheet.prixParEleveAvecSubventions,
        coutPrevisionnelParEleve: sheet.coutPrevisionnelParEleve,
        budgetValidatedAt: sheet.budgetValidatedAt,
        recettesEleves: sheet.recettesEleves,
        totalSubventions: sheet.totalSubventions,
      }
    : null;
  return {
    ...rest,
    comptaSheet: comptaLite,
    selectedBusQuote: data?.selectedBusQuote
      ? {
          providerName: data.selectedBusQuote.providerName,
          id: data.selectedBusQuote.id,
        }
      : null,
    participantElevesCount: Array.isArray(data?.participantEleves) ? data.participantEleves.length : 0,
    participantEleves: undefined,
  };
}

/**
 * Audit métier du séjour : étape, JSON utile, manques, conseils.
 * Destiné à ScolIA (outil + prompt d’aide).
 */
export function buildTripWorkflowAudit(trip: TravelsTrip): TripWorkflowAudit {
  const status = String(trip.status || "");
  const withBus = complexNeedsBus(trip);
  const d = trip.data || {};
  const devisList = Array.isArray(trip.receivedDevis) ? trip.receivedDevis : [];
  const providers = devisList
    .map((q) => String((q as { providerName?: string }).providerName || "").trim())
    .filter(Boolean);
  const selectedProvider = d.selectedBusQuote?.providerName
    ? String(d.selectedBusQuote.providerName)
    : null;
  const signed = Boolean(d.signedQuoteUrl);
  const pendingAmended = Boolean(d.pendingAmendedQuote);
  const bypassed = Boolean(d.transportPhaseBypassedAt);

  const coutTotalPrevu = num(d.coutTotal);
  const costPerStudent = num(d.costPerStudent);
  const finalTotalCost = num(d.finalTotalCost);
  const sheet = d.comptaSheet;
  const depensesTotal = num(sheet?.depensesTotal);
  const prixParEleveAnnonce = num(sheet?.prixParEleveAnnonce);
  const prixParEleveCalcule = num(sheet?.prixParEleveAvecSubventions ?? sheet?.coutPrevisionnelParEleve);
  const budgetValidated = Boolean(sheet?.budgetValidatedAt || finalTotalCost);

  const nbEleves = num(d.nbEleves);
  const nbAccompagnateurs = num(d.nbAccompagnateurs);
  const listeElevesCount = Array.isArray(d.participantEleves) ? d.participantEleves.length : 0;
  const listeElevesConfirmed = d.listeElevesStatus === "confirmed" && listeElevesCount > 0;

  const filled: string[] = [];
  const missingForCurrentStep: string[] = [];
  const blockers: string[] = [];
  const advice: string[] = [];
  const expectedActorsNow: string[] = [];

  if (d.title) filled.push("title");
  if (d.destination) filled.push("destination");
  if (d.etablissement) filled.push("etablissement");
  if (d.classes) filled.push("classes");
  if (d.date || d.startDate) filled.push("dates");
  if (nbEleves != null && nbEleves > 0) filled.push("nbEleves");
  if (coutTotalPrevu != null && coutTotalPrevu > 0) filled.push("coutTotalPrevu");
  if (costPerStudent != null && costPerStudent > 0) filled.push("costPerStudent");
  if (devisList.length > 0) filled.push(`devisRecus:${devisList.length}`);
  if (selectedProvider) filled.push(`devisChoisi:${selectedProvider}`);
  if (signed) filled.push("devisSigne");
  if (budgetValidated) filled.push("budgetValide");
  if (listeElevesConfirmed) filled.push("listeElevesConfirmee");

  // —— Par statut ——
  if (status === "EN_ATTENTE_DIR_INITIAL") {
    expectedActorsNow.push("direction");
    if (!d.title) missingForCurrentStep.push("titre du projet");
    if (!d.destination) missingForCurrentStep.push("destination");
    if (!(d.date || d.startDate)) missingForCurrentStep.push("date(s)");
    if (!d.etablissement) missingForCurrentStep.push("établissement");
    advice.push(
      "La direction doit ouvrir le panneau « Circuit de validation » et cliquer sur « Valider pédagogie » (ou refuser / demander des modifications).",
    );
    if (trip.type === "COMPLEX" && withBus) {
      advice.push("Après validation pédagogique, le dossier passera en logistique transport (devis bus).");
    } else if (trip.type === "SIMPLE") {
      advice.push("Sortie SIMPLE : après pédagogie, le dossier ira directement en finances (pas d’étape bus).");
    }
  } else if (status === "PROF_LOGISTICS") {
    if (!withBus) {
      expectedActorsNow.push("createur", "direction");
      advice.push("Sans bus : cliquer « Passer aux finances » (créateur ou direction).");
    } else if (pendingAmended) {
      expectedActorsNow.push("transporteur");
      blockers.push("Avenant transport en cours — nouveau devis pas encore reçu ou pas encore choisi.");
      advice.push("Attendre le nouveau devis dans l’onglet Transport, puis « Choisir » (créateur) ou « Choisir et signer » (direction).");
    } else if (devisList.length === 0) {
      expectedActorsNow.push("transporteur");
      missingForCurrentStep.push("au moins un devis bus dans receivedDevis");
      blockers.push("Aucun devis reçu : impossible de choisir / signer.");
      advice.push("Relancer les transporteurs ou attendre leurs réponses. Ne pas faire attendre la direction pour une signature : il n’y a encore rien à signer.");
    } else {
      expectedActorsNow.push("createur", "direction");
      missingForCurrentStep.push("sélection d’un devis (selectedBusQuote) — créateur « Choisir » ou direction « Choisir et signer »");
      blockers.push(
        `${devisList.length} devis reçu(s) (${providers.join(", ") || "transporteurs"}) mais aucun n’est encore choisi.`,
      );
      if (devisList.length === 1) {
        advice.push(
          "Un seul devis est arrivé : on peut le choisir si le délai est serré, ou attendre 24–48h s’il reste d’autres transporteurs sollicités.",
        );
      } else if (devisList.length === 2) {
        advice.push(
          "Deux devis sont là : souvent suffisant pour comparer. Conseiller de choisir sauf si un 3ᵉ transporteur a été clairement sollicité et doit encore répondre.",
        );
      } else {
        advice.push(
          `${devisList.length} devis reçus : assez pour décider. Conseiller de choisir maintenant plutôt que d’attendre un devis supplémentaire « au cas où ».`,
        );
      }
      advice.push(
        "Créateur → bouton « Choisir » (puis signature direction). Direction → bouton « Choisir et signer » (choix + commande d’un coup).",
      );
    }
  } else if (status === "EN_ATTENTE_BUS_SIGNATURE") {
    expectedActorsNow.push("direction");
    if (!selectedProvider) missingForCurrentStep.push("devis sélectionné");
    if (!signed) missingForCurrentStep.push("signature / commande direction (signedQuoteUrl)");
    blockers.push(
      selectedProvider
        ? `Devis de « ${selectedProvider} » choisi — signature direction manquante.`
        : "Étape signature mais aucun devis sélectionné (anomalie).",
    );
    advice.push(
      "La direction doit signer / commander dans Transport (ou via le circuit de validation). Le créateur n’a plus à « Choisir ».",
    );
  } else if (status === "EN_ATTENTE_COMPTA") {
    expectedActorsNow.push("comptabilite");
    if (coutTotalPrevu == null || coutTotalPrevu <= 0) {
      missingForCurrentStep.push("prix / budget général du projet (data.coutTotal)");
      blockers.push("Budget prévisionnel global (coutTotal) absent ou à 0.");
    }
    if (costPerStudent == null || costPerStudent <= 0) {
      missingForCurrentStep.push("prix par élève prévisionnel (data.costPerStudent)");
      blockers.push("Prix par élève (costPerStudent) absent ou à 0.");
    }
    if (!sheet) {
      missingForCurrentStep.push("fiche compta (data.comptaSheet) à compléter dans l’onglet Compta");
      blockers.push("Fiche Compta non démarrée / vide.");
    } else {
      if (depensesTotal == null || depensesTotal <= 0) {
        missingForCurrentStep.push("total des dépenses sur la fiche compta");
        blockers.push("Dépenses totales non renseignées sur la fiche Compta.");
      }
      if (prixParEleveAnnonce == null && (prixParEleveCalcule == null || prixParEleveCalcule <= 0)) {
        missingForCurrentStep.push("prix par élève (annoncé ou calculé) sur la fiche compta");
        blockers.push("Aucun prix par élève exploitable sur la fiche Compta.");
      }
    }
    if (!budgetValidated) {
      missingForCurrentStep.push("validation budget compta (budgetValidatedAt / finalTotalCost)");
      blockers.push("Budget non validé par la comptabilité.");
    }
    advice.push(
      "La compta doit ouvrir l’onglet Compta, renseigner/contrôler le budget général et le prix par élève, puis valider le budget.",
    );
    if ((coutTotalPrevu == null || coutTotalPrevu <= 0) && (costPerStudent == null || costPerStudent <= 0)) {
      advice.push(
        "Le dossier paraît bloqué côté chiffres : ni prix général du projet ni prix par élève. Demander à compléter ces montants avant validation.",
      );
    }
  } else if (status === "EN_ATTENTE_DIR_FINAL") {
    expectedActorsNow.push("direction");
    if (!budgetValidated) {
      blockers.push("Passage en validation finale sans budget clairement validé — à vérifier.");
    }
    advice.push(
      "Dernière étape : la direction clique sur « Validation finale » dans le circuit de validation.",
    );
  } else if (status === "BESOIN_MODIFICATION") {
    expectedActorsNow.push("createur");
    const note = typeof d.modificationRequestNote === "string" ? d.modificationRequestNote.trim() : "";
    if (note) blockers.push(`Motif modifications : ${note}`);
    else blockers.push("Modifications demandées (motif à lire sur la fiche / historique).");
    advice.push("Le créateur corrige le dossier puis le renvoie dans le circuit.");
  }

  // Rappels transverses utiles
  if (withBus && status === "VALIDE" && !listeElevesConfirmed) {
    advice.push("Séjour validé avec bus : penser à confirmer la liste nominative des élèves avant le départ.");
  }

  return {
    tripId: trip.id,
    title: String(d.title || "(sans titre)"),
    type: trip.type || "COMPLEX",
    status,
    statusLabel: statusLabel(status),
    withBusLogistics: withBus,
    expectedPath: expectedPathFor(trip, withBus),
    expectedActorsNow,
    filled,
    missingForCurrentStep,
    blockers,
    advice,
    devis: {
      count: devisList.length,
      providers,
      selectedProvider,
      signed,
      pendingAmended,
      bypassed,
    },
    finances: {
      coutTotalPrevu,
      costPerStudent,
      finalTotalCost,
      comptaSheetPresent: Boolean(sheet),
      budgetValidated,
      depensesTotal,
      prixParEleveAnnonce,
      prixParEleveCalcule,
    },
    effectif: {
      nbEleves,
      nbAccompagnateurs,
      listeElevesCount,
      listeElevesConfirmed,
    },
    projectSnapshot: {
      id: trip.id,
      type: trip.type,
      status,
      statusLabel: statusLabel(status),
      ownerName: trip.ownerName || null,
      ownerEmail: trip.ownerEmail || null,
      withBusLogistics: withBus,
      data: stripHeavy(d),
      receivedDevis: devisList.map((q) => ({
        id: (q as { id?: string }).id || null,
        providerName: (q as { providerName?: string }).providerName || null,
        source: (q as { source?: string }).source || null,
      })),
    },
  };
}

/** Texte + JSON pour coller dans un prompt ScolIA. */
export function formatTripAuditForAiPrompt(audit: TripWorkflowAudit): string {
  return (
    `AUDIT SÉJOUR SCOLAIRE (source de vérité — base-toi UNIQUEMENT là-dessus)\n` +
    `Titre: ${audit.title}\n` +
    `Type: ${audit.type} | Bus: ${audit.withBusLogistics ? "oui" : "non"}\n` +
    `Étape actuelle: ${audit.status} (${audit.statusLabel})\n` +
    `Parcours attendu: ${audit.expectedPath.join(" → ")}\n` +
    `Acteurs attendus maintenant: ${audit.expectedActorsNow.join(", ") || "—"}\n` +
    `Rempli: ${audit.filled.join(", ") || "—"}\n` +
    `Manque pour cette étape: ${audit.missingForCurrentStep.join(" ; ") || "rien de bloquant détecté"}\n` +
    `Freins: ${audit.blockers.join(" ; ") || "—"}\n` +
    `Conseils système: ${audit.advice.join(" | ") || "—"}\n` +
    `Devis: ${audit.devis.count} reçu(s)` +
    (audit.devis.providers.length ? ` [${audit.devis.providers.join(", ")}]` : "") +
    `; choisi=${audit.devis.selectedProvider || "non"}; signé=${audit.devis.signed ? "oui" : "non"}\n` +
    `Finances: coutTotal=${audit.finances.coutTotalPrevu ?? "∅"}, ` +
    `prix/élève=${audit.finances.costPerStudent ?? "∅"}, ` +
    `ficheCompta=${audit.finances.comptaSheetPresent ? "oui" : "non"}, ` +
    `budgetValidé=${audit.finances.budgetValidated ? "oui" : "non"}, ` +
    `dépenses=${audit.finances.depensesTotal ?? "∅"}\n` +
    `JSON compact:\n${JSON.stringify(audit.projectSnapshot)}`
  );
}
