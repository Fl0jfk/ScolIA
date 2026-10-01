/**
 * Catalogue des actions ScolIA — vision « tout gérable ».
 * Chaque ligne = une intention utilisateur → outil Brain (existant ou à brancher).
 * Les mutations passent toujours par needsConfirmation / needsChoices / needsFileUpload.
 */

export type BrainActionCoverage = "live" | "partial" | "todo";

export type BrainActionEntry = {
  id: string;
  intentFr: string;
  tool?: string;
  coverage: BrainActionCoverage;
  moduleId: string;
};

/** Source de vérité produit pour étendre le registre sans trous. */
export const BRAIN_ACTION_CATALOG: BrainActionEntry[] = [
  // —— Live ——
  { id: "nav-open", intentFr: "Ouvrir une page / un module", tool: "resolve_and_open", coverage: "live", moduleId: "dashboard-week-sheet" },
  { id: "eleve-search", intentFr: "Chercher un élève", tool: "search_eleves", coverage: "live", moduleId: "eleve-dossier" },
  { id: "eleve-open", intentFr: "Ouvrir dossier / docs inscription", tool: "open_eleve_dossier", coverage: "live", moduleId: "eleve-dossier" },
  { id: "eleve-regime", intentFr: "Changer régime (interne / DP / externe)", tool: "update_eleve_regime", coverage: "live", moduleId: "eleve-dossier" },
  { id: "eleve-pap-list", intentFr: "Lister élèves PAP/PAI par classe", tool: "list_eleves_filtered", coverage: "live", moduleId: "eleve-dossier" },
  { id: "accueil-absence", intentFr: "Déclarer absence/retard élève (accueil)", tool: "create_accueil_absence", coverage: "live", moduleId: "accueil-absences" },
  { id: "absence-self", intentFr: "Déclarer mon absence (prof/OGEC)", tool: "create_absence", coverage: "live", moduleId: "absences" },
  { id: "photo-create", intentFr: "Demande photocopie + PDF", tool: "create_photocopie_demand", coverage: "live", moduleId: "photocopies-couleur" },
  { id: "room-book", intentFr: "Réserver une salle", tool: "create_reservation", coverage: "live", moduleId: "prof-room" },
  { id: "request-create", intentFr: "Créer une demande staff", tool: "create_request", coverage: "live", moduleId: "requests-staff" },
  { id: "trip-create", intentFr: "Créer une sortie scolaire", tool: "create_trip", coverage: "live", moduleId: "travels" },
  { id: "trip-open", intentFr: "Ouvrir un séjour (choix si plusieurs)", tool: "open_trip", coverage: "live", moduleId: "travels" },
  { id: "trip-status", intentFr: "Statut / audit d’un séjour", tool: "get_trip_status", coverage: "live", moduleId: "travels" },
  { id: "hse-create", intentFr: "Demande HSE", tool: "create_hse_demand", coverage: "live", moduleId: "demandes-hse" },
  { id: "week-sheet", intentFr: "Feuille de semaine / actualité", tool: "get_week_sheet_today", coverage: "live", moduleId: "dashboard-week-sheet" },
  { id: "stages-overview", intentFr: "Vue stages", tool: "get_stages_overview", coverage: "partial", moduleId: "stages" },
  { id: "internat-status", intentFr: "Statut internat (agrégats)", tool: "get_internat_status", coverage: "partial", moduleId: "internat" },

  // —— À brancher (API déjà là) ——
  { id: "eleve-grille-repas", intentFr: "Modifier grille repas jour par jour", coverage: "todo", moduleId: "eleve-dossier" },
  { id: "eleve-create", intentFr: "Créer un élève / préinscription", coverage: "todo", moduleId: "eleve-dossier" },
  { id: "accueil-cancel", intentFr: "Annuler une absence accueil", coverage: "todo", moduleId: "accueil-absences" },
  { id: "internat-appel", intentFr: "Faire l’appel internat", coverage: "todo", moduleId: "internat" },
  { id: "internat-assign", intentFr: "Affecter une chambre", coverage: "todo", moduleId: "internat" },
  { id: "stages-sign", intentFr: "Relancer / signer convention", coverage: "todo", moduleId: "stages" },
  { id: "notes-saisie", intentFr: "Saisir une note", coverage: "todo", moduleId: "notes" },
  { id: "rdv-book", intentFr: "Gérer un RDV inscription", coverage: "todo", moduleId: "rdv-inscription" },
  { id: "docs-upload", intentFr: "Déposer un fichier cloud", coverage: "todo", moduleId: "documents" },
  { id: "rh-leave", intentFr: "Valider une absence RH en file", coverage: "todo", moduleId: "rh" },
  { id: "messaging-send", intentFr: "Envoyer un message interne", coverage: "todo", moduleId: "channels" },
];

export function brainActionsLive(): BrainActionEntry[] {
  return BRAIN_ACTION_CATALOG.filter((a) => a.coverage === "live");
}

export function brainActionsTodo(): BrainActionEntry[] {
  return BRAIN_ACTION_CATALOG.filter((a) => a.coverage === "todo");
}
