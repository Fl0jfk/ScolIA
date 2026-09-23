import type { ImpactItem } from "./types";

/** Tiroir A — règles produit déjà tranchées (ne pas négocier). */
export function buildTiroirA(): ImpactItem[] {
  return [
    {
      domaine: "vie_scolaire",
      constat:
        "La participation à la sortie n’est pas une ligne d’absence bulletin (`vs_absence_eleve`).",
      tiroir: "A",
    },
    {
      domaine: "occupancy",
      constat:
        "La présence « hors les murs » se lit via occupancy (`en_sortie`) sur les participants live ∩ dates.",
      tiroir: "A",
    },
    {
      domaine: "edt",
      constat:
        "Créneau dont tous les élèves attendus sont en sortie → signal VS uniquement. Aucune écriture `teacher_planning_replacement`.",
      tiroir: "A",
    },
    {
      domaine: "rh",
      constat: "AESH / ATSEM = autre employeur — pas d’affectation OGEC automatique.",
      tiroir: "A",
    },
    {
      domaine: "ia",
      constat: "L’IA propose ; toute écriture métier exige un clic humain.",
      tiroir: "A",
    },
  ];
}

/** Tiroir B — conséquences déterministes si on ignore le séjour. */
export function buildTiroirB(opts: {
  participantCount: number;
  participantLinkedCount: number;
  panierRepasCount: number;
  creneauxVidesCount: number;
  status: string;
}): ImpactItem[] {
  const items: ImpactItem[] = [
    {
      domaine: "appel",
      constat:
        opts.participantLinkedCount > 0
          ? `${opts.participantLinkedCount} élève(s) lié(s) hors les murs — l’appel / « où est X » doit lire occupancy, pas inventer une absence.`
          : opts.participantCount > 0
            ? `${opts.participantCount} participant(s) sans eleve_id — occupancy partielle ; matching à revoir.`
            : "Aucun participant nominatif — pas d’effet présence calculable.",
      tiroir: "B",
    },
    {
      domaine: "edt",
      constat:
        opts.creneauxVidesCount > 0
          ? `${opts.creneauxVidesCount} créneau(x) potentiellement vidé(s) (tous les attendus en sortie) — signal VS listé, pas de remplacement inventé.`
          : "Aucun créneau 100 % vidé détecté (classe partielle ou pas d’EDT / population).",
      tiroir: "B",
    },
    {
      domaine: "restauration",
      constat:
        opts.panierRepasCount > 0
          ? `${opts.panierRepasCount} panier(s) repas déjà dans le dossier (commande ≠ 0 repas établissement).`
          : "Aucun panier repas coché sur la liste — la cantine établissement n’est pas prévenue automatiquement.",
      tiroir: "B",
    },
  ];

  if (opts.status === "ANNULE" || opts.status === "SEANCE_ANNULEE") {
    items.push({
      domaine: "occupancy",
      constat:
        "Annulation : la participation live disparaît ; rien à « dé-deleter » côté bulletin (aucune absence sortie n’avait été écrite).",
      tiroir: "B",
    });
  }

  return items;
}

/** Tiroir C — règles locales absentes → questions bornées, pas d’écriture. */
export function buildTiroirC(): ImpactItem[] {
  return [
    {
      domaine: "vie_scolaire",
      constat: "Élèves non participants de la classe / du groupe.",
      tiroir: "C",
      question: "Cours, regroupement, étude, autre ? Règle locale absente.",
    },
    {
      domaine: "internat",
      constat: "Interne en séjour le soir.",
      tiroir: "C",
      question: "Excuse / activité / absent / autre sur l’appel internat ? (`activite` internat ≠ voyage.)",
    },
    {
      domaine: "restauration",
      constat: "Impact cantine établissement.",
      tiroir: "C",
      question: "Déprévision, forfait, paniers seulement, mixte ?",
    },
    {
      domaine: "rh",
      constat: "Accompagnateurs staff.",
      tiroir: "C",
      question: "Mission, HSE, heures, autre ?",
    },
    {
      domaine: "facturation",
      constat: "Facturation famille du voyage.",
      tiroir: "C",
      question: "Règle tarif / émission à définir localement.",
    },
    {
      domaine: "sante",
      constat: "PAI / médicaments emportés.",
      tiroir: "C",
      question: "Diffusion opérationnelle autorisée ? Extraction non codée.",
    },
    {
      domaine: "travels",
      constat: "Liste après gel transporteur.",
      tiroir: "C",
      question: "Politique de modification / delta CSV ?",
    },
  ];
}

/** Tiroir D — module / donnée / droit absent → unavailable. */
export function buildTiroirD(opts?: { edtCoverage?: "complete" | "partial" | "unavailable" }): ImpactItem[] {
  return [
    {
      domaine: "internat",
      constat: "Roster nominatif soir encore EAV / partiel.",
      tiroir: "D",
      coverage: "unavailable",
      question: "Impossible de projeter l’appel internat nominatif depuis ce moteur.",
    },
    {
      domaine: "restauration",
      constat: "Ops self / menus absents.",
      tiroir: "D",
      coverage: "unavailable",
      question: "Pas de prévision passage cantine opposable.",
    },
    {
      domaine: "paie",
      constat: "Paie hors périmètre runtime.",
      tiroir: "D",
      coverage: "unavailable",
    },
    {
      domaine: "messagerie_familles",
      constat:
        "Canal dédié foyer (famille_thread) — Value Gate light ; Messenger staff reste cloisonné.",
      tiroir: "D",
      coverage: "partial",
    },
    {
      domaine: "edt",
      constat:
        opts?.edtCoverage === "unavailable"
          ? "Aucun créneau EDT pour les classes/groupes concernés — grille non opposable ou vide."
          : "Source EDT (saisie / import / solveur) encore à consolider ; détection best-effort sur `edt_creneau`.",
      tiroir: "D",
      coverage: opts?.edtCoverage ?? "partial",
    },
  ];
}

export function assembleImpacts(opts: {
  participantCount: number;
  participantLinkedCount: number;
  panierRepasCount: number;
  creneauxVidesCount: number;
  status: string;
  edtCoverage?: "complete" | "partial" | "unavailable";
}): ImpactItem[] {
  return [
    ...buildTiroirA(),
    ...buildTiroirB(opts),
    ...buildTiroirC(),
    ...buildTiroirD({ edtCoverage: opts.edtCoverage }),
  ];
}
