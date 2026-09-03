/**
 * Defaults métier explicites (source de vérité hors overrides admin).
 * Si un rôle n’a pas d’entrée ici, on retombe sur `allowedRoles` du catalogue.
 * Fichier sans dépendance server-only (utilisable côté client UI).
 */

export type DefaultDossierSection =
  | "identite"
  | "scolarite"
  | "famille"
  | "documents"
  | "notes"
  | "vie_scolaire"
  | "sante"
  | "facturation";

/** Dossier élève complet (toutes sections). */
export const DOSSIER_SECTIONS_COMPLET: readonly DefaultDossierSection[] = [
  "identite",
  "scolarite",
  "famille",
  "documents",
  "notes",
  "vie_scolaire",
  "sante",
  "facturation",
];

/** Modules par défaut pour les 3 rôles direction. */
export const DIRECTION_DEFAULT_MODULES: readonly string[] = [
  // Administratif
  "eleve-dossier",
  "agent-ia-ocr",
  "stages",
  "certificates",
  "notes",
  "groupes-pedagogiques",
  // Établissement
  "admin-settings",
  "organigramme",
  "evenements",
  "communication",
  "conformite-rgpd",
  // Services
  "documents",
  "travels",
  "qrcreator",
  "prof-room",
  "domain-planning",
  "channels",
  "requests-staff",
  "photocopies-couleur",
  "assistance",
  // Vie scolaire
  "internat",
  "vs-calendrier",
  "accueil-absences",
  "accueil-portes-ouvertes",
  // Compta & RH
  "rh",
  "absences",
  "demandes-hse",
  "mon-planning",
];

export const ROLE_DEFAULT_MODULES: Record<string, readonly string[]> = {
  professeur: [
    // Pas de boîte à outils ni vie scolaire (appels / absences élèves / carnet) par défaut.
    "eleve-dossier",
    "certificates",
    "organigramme",
    "documents",
    "travels",
    "prof-room",
    "domain-planning",
    "channels",
    "requests-staff",
    "photocopies-couleur",
    "assistance",
    "vs-calendrier",
    "rh",
    "absences",
    "demandes-hse",
  ],
  administratif: [
    "eleve-dossier",
    "agent-ia-ocr",
    "stages",
    "certificates",
    "notes",
    "groupes-pedagogiques",
    "organigramme",
    "evenements",
    "communication",
    "documents",
    "travels",
    "prof-room",
    "domain-planning",
    "channels",
    "requests-staff",
    "photocopies-couleur",
    "assistance",
    "internat",
    "vs-calendrier",
    "accueil-absences",
    "accueil-portes-ouvertes",
    "rh",
    "absences",
    "mon-planning",
  ],
  /**
   * Standard / accueil : périmètre réduit.
   * Dossiers élèves = liste seule ; sorties = visu globale (pas de fiche).
   * Photocopies : réceptionnaire par défaut (voir ROLE_DEFAULT_PHOTOCOPIES_OPS).
   */
  accueil: [
    "accueil-absences",
    "accueil-portes-ouvertes",
    "eleve-dossier",
    "travels",
    "photocopies-couleur",
    "rh",
    "absences",
    "mon-planning",
  ],
  direction_ecole: DIRECTION_DEFAULT_MODULES,
  direction_college: DIRECTION_DEFAULT_MODULES,
  direction_lycee: DIRECTION_DEFAULT_MODULES,
  comptabilite: [
    "eleve-dossier",
    "organigramme",
    "documents",
    "travels",
    "channels",
    "requests-staff",
    "assistance",
    "rh",
    "absences",
    "mon-planning",
  ],
  maintenance: [
    "organigramme",
    "documents",
    "prof-room",
    "requests-staff",
    "channels",
    "assistance",
    "rh",
    "absences",
    "mon-planning",
  ],
  infirmerie: [
    "eleve-dossier",
    "sante",
    "organigramme",
    "documents",
    "travels",
    "domain-planning",
    "requests-staff",
    "assistance",
    "internat",
    "rh",
    "absences",
    "mon-planning",
  ],
  psychologue: [
    "eleve-dossier",
    "sante",
    "organigramme",
    "documents",
    "domain-planning",
    "requests-staff",
    "rh",
    "absences",
    "mon-planning",
  ],
  surveillant: [
    "eleve-dossier",
    "evenements",
    "organigramme",
    "documents",
    "channels",
    "assistance",
    "requests-staff",
    "internat",
    "absences-accueil-consultation",
    "rh",
    "absences",
    "mon-planning",
  ],
  cpe: [
    "eleve-dossier",
    "stages",
    "certificates",
    "notes",
    "groupes-pedagogiques",
    "organigramme",
    "documents",
    "travels",
    "prof-room",
    "domain-planning",
    "channels",
    "requests-staff",
    "assistance",
    "internat",
    "vs-calendrier",
    "accueil-absences",
    "absences-accueil-consultation",
    "rh",
    "absences",
    "mon-planning",
  ],
  internat: [
    "eleve-dossier",
    "organigramme",
    "documents",
    "channels",
    "assistance",
    "requests-staff",
    "internat",
    "vs-calendrier",
    "vs-appels",
    "vs-sanctions",
    "vs-carnet",
    "rh",
    "absences",
    "mon-planning",
  ],
};

/** Rôles réceptionnaires photocopies couleur par défaut (file « à imprimer »). */
export const ROLE_DEFAULT_PHOTOCOPIES_OPS: ReadonlySet<string> = new Set(["accueil"]);

export function roleHasDefaultPhotocopiesOps(role: string): boolean {
  return ROLE_DEFAULT_PHOTOCOPIES_OPS.has(role);
}

export function rolesHaveDefaultPhotocopiesOps(roles: string[]): boolean {
  return roles.some((r) => ROLE_DEFAULT_PHOTOCOPIES_OPS.has(r));
}

export const ROLE_DEFAULT_DOSSIER_SECTIONS: Record<string, readonly DefaultDossierSection[]> = {
  professeur: ["identite", "scolarite", "notes", "documents", "vie_scolaire"],
  /** Accueil : pas d’ouverture de fiche ; sections vides si accès détail forcé. */
  accueil: [],
  administratif: [
    "identite",
    "scolarite",
    "famille",
    "documents",
    "notes",
    "vie_scolaire",
    "facturation",
  ],
  direction_ecole: DOSSIER_SECTIONS_COMPLET,
  direction_college: DOSSIER_SECTIONS_COMPLET,
  direction_lycee: DOSSIER_SECTIONS_COMPLET,
  comptabilite: ["identite", "scolarite", "famille", "facturation", "documents"],
  infirmerie: ["identite", "scolarite", "famille", "sante", "documents"],
  psychologue: ["identite", "scolarite"],
  surveillant: ["identite", "scolarite", "famille"],
  /** Documents : catégorie administratif uniquement (pas financier / santé). */
  cpe: ["identite", "scolarite", "famille", "documents", "notes", "vie_scolaire"],
  internat: ["identite", "scolarite", "famille", "vie_scolaire"],
};

export function hasCustomRoleDefaults(role: string): boolean {
  return Object.prototype.hasOwnProperty.call(ROLE_DEFAULT_MODULES, role);
}

export function customDefaultModulesForRole(role: string): string[] | null {
  const list = ROLE_DEFAULT_MODULES[role];
  return list ? [...list] : null;
}

export function customDefaultDossierSectionsForRole(role: string): DefaultDossierSection[] | null {
  const list = ROLE_DEFAULT_DOSSIER_SECTIONS[role];
  return list ? [...list] : null;
}
