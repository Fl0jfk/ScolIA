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

export const ROLE_DEFAULT_MODULES: Record<string, readonly string[]> = {
  professeur: [
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
    "vs-carnet",
    "vs-appels",
    "vs-calendrier",
    "rh",
    "absences",
    "demandes-hse",
  ],
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
  infirmerie: ["eleve-dossier", "sante", "organigramme", "assistance"],
};

export const ROLE_DEFAULT_DOSSIER_SECTIONS: Record<string, readonly DefaultDossierSection[]> = {
  professeur: ["identite", "scolarite", "notes"],
  comptabilite: ["identite", "scolarite", "famille", "facturation", "documents"],
  infirmerie: ["identite", "scolarite", "famille", "sante", "documents"],
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
