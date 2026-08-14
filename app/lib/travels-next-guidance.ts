import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import { TRAVELS_STATUS_LABELS, type TravelsHubTab, type TravelsTrip } from "@/app/lib/travels-types";
import {
  buildTripWorkflowAudit,
  formatTripAuditForAiPrompt,
} from "@/app/lib/travels-workflow-audit";

export type TripNextGuidance = {
  /** Libellé d’étape (ex. Choix du devis transport). */
  stepLabel: string;
  /** Qui doit agir (rôle métier). */
  who: string;
  /** Phrase courte / accroche. */
  headline: string;
  /** Explication claire, une ou deux phrases. */
  what: string;
  /** Checklist concrète (boutons, onglets, ordre). */
  steps: string[];
  /** L’utilisateur connecté est celui qui doit agir. */
  youMustAct: boolean;
  /** Si ce n’est pas à vous : ce que vous pouvez faire en attendant. */
  whileWaiting?: string;
  /** Onglet à ouvrir pour débloquer. */
  ctaTab?: TravelsHubTab;
  ctaLabel?: string;
  /** Prompt prêt à envoyer à ScolIA (inclut l’audit JSON du séjour). */
  aiPrompt: string;
};

type GuidanceCtx = {
  isOwner: boolean;
  canSign: boolean;
  isCompta: boolean;
};

function statusLabel(status: string): string {
  return TRAVELS_STATUS_LABELS[status] || status;
}

function buildAiPrompt(trip: TravelsTrip, focusQuestion: string): string {
  const audit = buildTripWorkflowAudit(trip);
  return (
    `Tu analyses ce séjour scolaire. ${focusQuestion}\n\n` +
    `Consignes :\n` +
    `- Distingue bien sortie SIMPLE et séjour COMPLEX (bus ou non).\n` +
    `- Dis qui doit agir maintenant (créateur / direction / compta / transporteur).\n` +
    `- Liste ce qui est déjà fait vs ce qui manque d’après l’audit.\n` +
    `- Si des devis sont là : conseille d’attendre ou de choisir selon le nombre et le contexte.\n` +
    `- Si étape finances : cite les montants manquants (budget global, prix/élève, validation).\n` +
    `- Réponds en français, concret, étapes numérotées. Ne invente pas de champs absents du JSON.\n\n` +
    formatTripAuditForAiPrompt(audit)
  );
}

/**
 * Texte « Qui doit faire quoi » selon le statut du séjour.
 * Retourne null pour les états terminaux (validé / annulé / refusé).
 */
export function getTripNextGuidance(trip: TravelsTrip, ctx: GuidanceCtx): TripNextGuidance | null {
  const { isOwner, canSign, isCompta } = ctx;
  const status = String(trip.status || "");
  const withBus = complexNeedsBus(trip);
  const devisCount = Array.isArray(trip.receivedDevis) ? trip.receivedDevis.length : 0;
  const pendingAmended = Boolean(trip.data?.pendingAmendedQuote);
  const stepLabel = statusLabel(status);
  const audit = buildTripWorkflowAudit(trip);
  const missingSteps =
    audit.missingForCurrentStep.length > 0
      ? audit.missingForCurrentStep.map((m) => `À compléter : ${m}`)
      : [];
  const blockerNotes = audit.blockers.slice(0, 3);

  const finish = (
    g: Omit<TripNextGuidance, "aiPrompt"> & { focusQuestion: string },
  ): TripNextGuidance => ({
    stepLabel: g.stepLabel,
    who: g.who,
    headline: g.headline,
    what: blockerNotes.length ? `${g.what} (${blockerNotes.join(" · ")})` : g.what,
    steps: [
      ...g.steps,
      ...missingSteps.filter(
        (s) => !g.steps.some((x) => x.toLowerCase().includes(s.replace(/^À compléter : /i, "").slice(0, 24).toLowerCase())),
      ),
    ],
    youMustAct: g.youMustAct,
    whileWaiting: g.whileWaiting,
    ctaTab: g.ctaTab,
    ctaLabel: g.ctaLabel,
    aiPrompt: buildAiPrompt(trip, g.focusQuestion),
  });

  if (status === "VALIDE" || status === "REJETE" || status === "ANNULE" || status === "SEANCE_ANNULEE") {
    return null;
  }

  if (status === "BESOIN_MODIFICATION") {
    return finish({
      stepLabel,
      who: "Créateur du séjour (professeur organisateur)",
      headline: isOwner
        ? "C’est à vous de modifier le dossier, puis de le renvoyer."
        : "Le créateur doit corriger le dossier avant de le renvoyer.",
      what: "La direction (ou la compta) a demandé des changements. Tant qu’ils ne sont pas faits et renvoyés, le circuit reste bloqué ici.",
      steps: isOwner
        ? [
            "Lisez le motif de modification affiché sur cette page.",
            "Corrigez les champs concernés (dates, effectifs, budget, pièces jointes…).",
            "Enregistrez, puis renvoyez le dossier dans le circuit via le panneau de décision / les boutons d’action.",
          ]
        : [
            "Le créateur voit le motif de modification sur sa fiche séjour.",
            "Il doit corriger puis renvoyer le dossier.",
            "Vous serez à nouveau sollicité(e) pour valider ensuite.",
          ],
      youMustAct: isOwner,
      whileWaiting: isOwner
        ? undefined
        : "Vous pouvez relancer le créateur si besoin ; vous n’avez rien à valider tant que le dossier n’est pas renvoyé.",
      ctaTab: "overview",
      ctaLabel: isOwner ? "Voir la demande" : "Voir le dossier",
      focusQuestion: "Des modifications ont été demandées. Que doit faire exactement le créateur pour débloquer ?",
    });
  }

  if (status === "EN_ATTENTE_DIR_INITIAL") {
    return finish({
      stepLabel,
      who: "Direction de l’établissement concerné",
      headline: canSign
        ? "C’est à la direction de valider (ou refuser) la pédagogie."
        : "En attente de la direction : validation pédagogique.",
      what: canSign
        ? "Sans votre validation pédagogique, le séjour ne peut pas avancer (ni logistique bus, ni finances)."
        : "Le projet est créé. Seule la direction peut cliquer sur « Valider pédagogie » (ou refuser / demander des modifications).",
      steps: canSign
        ? [
            "Lisez le projet (destination, classes, dates, budget prévu) dans la vue d’ensemble.",
            "Descendez au panneau sombre « Circuit de validation / Espace décisionnaire ».",
            "Cliquez sur « Valider pédagogie » pour faire avancer le dossier — ou « Refus définitif » / demande de modifications si besoin.",
          ]
        : [
            "La direction ouvre ce séjour.",
            "Elle utilise le panneau « Circuit de validation » en bas de la vue d’ensemble.",
            "Elle clique sur « Valider pédagogie » (bouton principal) pour débloquer la suite.",
          ],
      youMustAct: canSign,
      whileWaiting: canSign
        ? undefined
        : "En tant que créateur, vous n’avez rien à cliquer ici : attendez la direction. Vous pouvez compléter documents / effectifs en parallèle si besoin.",
      ctaTab: "overview",
      ctaLabel: canSign ? "Aller à la décision" : "Voir le dossier",
      focusQuestion:
        "Le dossier attend la validation pédagogique direction. Que doit faire la direction concrètement ?",
    });
  }

  if (status === "PROF_LOGISTICS") {
    if (!withBus) {
      return finish({
        stepLabel,
        who: "Créateur ou direction",
        headline: "Pas de bus : il faut passer manuellement aux finances.",
        what: "Ce séjour est marqué sans transport bus. L’étape devis est ignorée, mais quelqu’un doit cliquer pour envoyer le dossier en compta.",
        steps: [
          "Ouvrez la vue d’ensemble.",
          "Cliquez sur « Passer aux finances » (créateur ou direction).",
          "La comptabilité pourra ensuite valider le budget.",
        ],
        youMustAct: isOwner || canSign,
        ctaTab: "overview",
        ctaLabel: "Voir les actions",
        focusQuestion: "Il n’y a pas de bus. Comment passer aux finances ?",
      });
    }
    if (pendingAmended) {
      return finish({
        stepLabel,
        who: "Transporteur, puis créateur ou direction",
        headline: "Avenant envoyé : on attend un nouveau devis.",
        what: "Un avenant a été demandé au transporteur. Quand le nouveau devis arrive, le créateur peut cliquer sur « Choisir », ou la direction sur « Choisir et signer ».",
        steps: [
          "Suivez l’onglet Transport pour voir si le nouveau devis est arrivé.",
          "Dès qu’il apparaît : créateur → « Choisir », ou direction → « Choisir et signer ».",
          "Si seul le créateur a choisi, la direction devra encore signer ensuite.",
        ],
        youMustAct: isOwner || canSign,
        whileWaiting:
          isOwner || canSign
            ? undefined
            : "Rien à signer pour l’instant : patience côté transporteur.",
        ctaTab: "transport",
        ctaLabel: "Voir le transport",
        focusQuestion: "Un avenant transport est en cours. Qui fait quoi ensuite ?",
      });
    }
    if (devisCount === 0) {
      return finish({
        stepLabel,
        who: "Transporteurs d’abord, puis créateur ou direction",
        headline: "Aucun devis reçu pour l’instant.",
        what: "Les transporteurs doivent renvoyer leurs devis. Dès qu’un devis apparaît dans Transport, le créateur peut le « Choisir », ou la direction peut « Choisir et signer » en une seule fois.",
        steps: [
          "Ouvrez l’onglet Transport pour vérifier l’état des demandes de devis.",
          "Attendez qu’au moins un devis apparaisse dans la liste.",
          "Ensuite : créateur → bouton « Choisir », ou direction → bouton « Choisir et signer ».",
        ],
        youMustAct: false,
        whileWaiting: isOwner
          ? "Ce n’est pas encore le moment de choisir : il faut d’abord un devis dans la liste. Vous pouvez relancer les transporteurs si besoin."
          : canSign
            ? "Tant qu’aucun devis n’est listé, le bouton « Choisir et signer » n’est pas utilisable."
            : "Créateur ou direction devront choisir un devis dès qu’il sera reçu.",
        ctaTab: "transport",
        ctaLabel: "Ouvrir Transport",
        focusQuestion: "On est à l’étape choix du devis transport mais aucun devis n’est encore là. Que faut-il faire ?",
      });
    }
    return finish({
      stepLabel,
      who: "Créateur ou direction",
      headline:
        isOwner && !canSign
          ? `À vous (créateur) : choisissez un devis (${devisCount} reçu${devisCount > 1 ? "s" : ""}).`
          : canSign
            ? `À vous (direction) : vous pouvez choisir et signer un devis (${devisCount} reçu${devisCount > 1 ? "s" : ""}).`
            : `${devisCount} devis reçu${devisCount > 1 ? "s" : ""} — en attente d’un choix.`,
      what:
        isOwner && !canSign
          ? "Ouvrez Transport et cliquez sur « Choisir » sur le devis retenu. Ensuite la direction devra encore signer la commande."
          : canSign
            ? "Ouvrez Transport : le bouton « Choisir et signer » retient le devis et lance la signature / commande en une fois. (Le créateur peut aussi seulement « Choisir », auquel cas il vous restera la signature.)"
            : "Soit le créateur clique sur « Choisir », soit la direction clique sur « Choisir et signer ».",
      steps: [
        ...(canSign
          ? [
              "Ouvrez l’onglet Transport.",
              "Comparez les devis reçus (prix, prestations).",
              "Cliquez sur « Choisir et signer » sur le devis retenu (choix + signature / commande).",
              "Alternative : si le créateur a déjà cliqué « Choisir », signez ensuite à l’étape signature.",
            ]
          : isOwner
            ? [
                "Ouvrez l’onglet Transport.",
                "Comparez les devis reçus (prix, prestations).",
                "Cliquez sur « Choisir » sur le devis retenu.",
                "La direction devra ensuite signer / commander (sauf si elle le fait elle-même via « Choisir et signer »).",
              ]
            : [
                "Le créateur peut cliquer sur « Choisir » dans Transport.",
                "Ou la direction peut cliquer sur « Choisir et signer » (choix + signature d’un coup).",
                "Sans l’un de ces deux gestes, le dossier reste bloqué en logistique.",
              ]),
        ...audit.advice.slice(0, 2),
      ],
      youMustAct: isOwner || canSign,
      whileWaiting:
        isOwner || canSign
          ? undefined
          : "En attente du créateur (« Choisir ») ou de la direction (« Choisir et signer »).",
      ctaTab: "transport",
      ctaLabel: canSign ? "Choisir et signer" : isOwner ? "Choisir un devis" : "Voir les devis",
      focusQuestion:
        "Des devis sont arrivés. Le créateur peut « Choisir », la direction peut « Choisir et signer ». Faut-il attendre d’autres devis ou choisir maintenant ?",
    });
  }

  if (status === "EN_ATTENTE_BUS_SIGNATURE") {
    return finish({
      stepLabel,
      who: "Direction",
      headline: canSign
        ? "C’est à la direction de signer et commander le devis bus."
        : "Devis choisi — en attente de signature direction.",
      what: canSign
        ? "Le créateur a fait sa part. Sans votre signature / commande, le dossier n’ira pas en finances."
        : "Vous (créateur) n’avez plus rien à choisir : la direction doit ouvrir Transport et signer le devis retenu.",
      steps: canSign
        ? [
            "Ouvrez l’onglet Transport (le devis choisi y est mis en avant).",
            "Vérifiez le devis, puis utilisez l’action de signature / commande.",
            "Vous pouvez aussi passer par le panneau « Circuit de validation » si le bouton y est proposé.",
            "Après signature, le dossier part en validation finances (compta).",
          ]
        : [
            "La direction ouvre Transport.",
            "Elle signe / commande le devis sélectionné.",
            "Le créateur attend simplement cette signature.",
          ],
      youMustAct: canSign,
      whileWaiting: canSign
        ? undefined
        : "Rien à cliquer de votre côté pour débloquer — sauf relancer la direction si besoin.",
      ctaTab: "transport",
      ctaLabel: canSign ? "Signer le devis" : "Voir le devis choisi",
      focusQuestion: "Le devis est choisi. Que doit faire exactement la direction pour signer et débloquer ?",
    });
  }

  if (status === "EN_ATTENTE_COMPTA") {
    return finish({
      stepLabel,
      who: "Comptabilité",
      headline: isCompta
        ? "C’est à la compta de valider le budget / la fiche finances."
        : "En attente de la comptabilité.",
      what: isCompta
        ? "Sans validation budget, la direction ne pourra pas faire la validation finale."
        : "Le transport (si besoin) est passé. Seule la compta peut valider les finances pour débloquer la suite.",
      steps: isCompta
        ? [
            "Ouvrez l’onglet Compta.",
            "Complétez / vérifiez la fiche budget (OCR devis, montants, aides…).",
            "Contrôlez le budget général du projet et le prix par élève.",
            "Validez le budget (bouton de validation compta).",
            "Le dossier passera ensuite en validation finale direction.",
          ]
        : [
            "La compta ouvre l’onglet Compta du séjour.",
            "Elle complète les montants manquants (budget global, prix/élève) puis valide.",
            "Ensuite la direction fera la validation finale.",
          ],
      youMustAct: isCompta,
      whileWaiting: isCompta
        ? undefined
        : "Créateur / direction : pas de bouton de validation budget de votre côté à cette étape.",
      ctaTab: "compta",
      ctaLabel: isCompta ? "Ouvrir Compta" : "Voir Compta",
      focusQuestion:
        "Le séjour est en validation finances. Qu’est-ce qui manque dans le JSON (prix global, prix/élève, fiche compta) et que doit faire la compta ?",
    });
  }

  if (status === "EN_ATTENTE_DIR_FINAL") {
    return finish({
      stepLabel,
      who: "Direction",
      headline: canSign
        ? "Dernière étape : validation finale par la direction."
        : "Budget OK — en attente de la validation finale direction.",
      what: canSign
        ? "Tout le circuit amont est passé. Cliquez sur « Validation finale » pour clôturer / finaliser le séjour."
        : "La compta a validé. Il reste uniquement le feu vert final de la direction.",
      steps: canSign
        ? [
            "Vérifiez rapidement le dossier (effectifs, transport, budget).",
            "Descendez au panneau « Circuit de validation ».",
            "Cliquez sur « Validation finale ».",
          ]
        : [
            "La direction ouvre le panneau « Circuit de validation ».",
            "Elle clique sur « Validation finale ».",
          ],
      youMustAct: canSign,
      whileWaiting: canSign
        ? undefined
        : "Rien d’autre à valider de votre côté tant que la direction n’a pas cliqué.",
      ctaTab: "overview",
      ctaLabel: canSign ? "Aller à la décision" : "Voir le dossier",
      focusQuestion: "Le séjour attend la validation finale. Que doit faire la direction ?",
    });
  }

  return null;
}

/** Résumé neutre (sans rôle utilisateur) — utile pour l’outil Brain AI. */
export function describeTripWorkflowForAi(trip: TravelsTrip): string {
  const audit = buildTripWorkflowAudit(trip);
  const g = getTripNextGuidance(trip, { isOwner: false, canSign: false, isCompta: false });
  if (!g) {
    return `Statut terminal : ${audit.statusLabel}.`;
  }
  return (
    `Étape : ${g.stepLabel}. Qui agit : ${g.who}. ${g.headline} ` +
    `Manques: ${audit.missingForCurrentStep.join("; ") || "—"}. ` +
    `Freins: ${audit.blockers.join("; ") || "—"}. ` +
    `Conseils: ${audit.advice.join(" | ")}`
  );
}
