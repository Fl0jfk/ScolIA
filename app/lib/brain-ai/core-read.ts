/**
 * Mise en forme des ports de lecture IA (14.5) — pur, testable.
 * Pas de SQL ici. Les handlers appellent les libs puis passent ici.
 */

import {
  ROLE_DEFAULT_DOSSIER_SECTIONS,
  type DefaultDossierSection,
} from "@/app/lib/module-access-defaults";
import type { OccupancyTag } from "@/app/lib/occupancy/types";

export type CoreReadCoverage = "complete" | "partial" | "unavailable";
export type CoreReadConfidence = "defined" | "deduced" | "unknown";

export type CoreReadEnvelope<T> = {
  facts: T;
  coverage: CoreReadCoverage;
  confidenceTag: CoreReadConfidence;
};

export function envelope<T>(
  facts: T,
  coverage: CoreReadCoverage = "complete",
  confidenceTag: CoreReadConfidence = "defined",
): CoreReadEnvelope<T> {
  return { facts, coverage, confidenceTag };
}

const ELEVATED_ELEVE_ROLES = new Set([
  "administratif",
  "accueil",
  "comptabilite",
  "surveillant",
  "cpe",
  "infirmerie",
  "psychologue",
  "internat",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
]);

export function dossierSectionsForRoles(
  roles: string[],
  isOrgAdmin: boolean,
): Set<DefaultDossierSection> {
  if (isOrgAdmin) {
    return new Set<DefaultDossierSection>([
      "identite",
      "scolarite",
      "famille",
      "documents",
      "notes",
      "vie_scolaire",
      "facturation",
      "sante",
    ]);
  }
  const out = new Set<DefaultDossierSection>();
  for (const role of roles) {
    const list = ROLE_DEFAULT_DOSSIER_SECTIONS[role];
    if (!list) continue;
    for (const s of list) out.add(s);
  }
  return out;
}

/** Accueil / rôles sans section famille → pas de foyer / e-mails parents. */
export function canSeeFamille(roles: string[], isOrgAdmin: boolean): boolean {
  return dossierSectionsForRoles(roles, isOrgAdmin).has("famille");
}

/**
 * `null` = pas de filtre classe (direction / admin / cpe…).
 * Tableau (éventuellement vide) = professeur restreint à ces classes.
 */
export function professorClassRestriction(
  roles: string[],
  isOrgAdmin: boolean,
): string[] | null {
  if (isOrgAdmin) return null;
  const elevated = roles.some((r) => ELEVATED_ELEVE_ROLES.has(r));
  if (elevated) return null;
  if (!roles.includes("professeur")) return null;
  return []; // à remplir par le handler via EDT / arg classe
}

export function eleveAllowedByClassRestriction(
  eleveClasse: string | null | undefined,
  allowedClasses: string[] | null,
): boolean {
  if (allowedClasses === null) return true;
  const c = (eleveClasse || "").trim();
  if (!c) return false;
  const norm = (s: string) => s.trim().toLowerCase();
  return allowedClasses.some((a) => norm(a) === norm(c));
}

const FAMILLE_KEYS = [
  "parentEmail",
  "parent1Email",
  "parent2Email",
  "parentPhone",
  "parent1Phone",
  "parent2Phone",
] as const;

export type ElevePublicFact = {
  id: string | null;
  ine: string | null;
  nom: string;
  prenom: string;
  classe: string | null;
  regime: string | null;
  status: string | null;
  parentEmail?: string | null;
  parent1Email?: string | null;
  parent2Email?: string | null;
  parentPhone?: string | null;
  parent1Phone?: string | null;
  parent2Phone?: string | null;
};

export function shapeEleveForRoles(
  raw: {
    id?: string | null;
    ine?: string | null;
    nom: string;
    prenom: string;
    classe?: string | null;
    regime?: string | null;
    status?: string | null;
    parentEmail?: string | null;
    parent1Email?: string | null;
    parent2Email?: string | null;
    parentPhone?: string | null;
    parent1Phone?: string | null;
    parent2Phone?: string | null;
  },
  roles: string[],
  isOrgAdmin: boolean,
): ElevePublicFact {
  const sections = dossierSectionsForRoles(roles, isOrgAdmin);
  const base: ElevePublicFact = {
    id: raw.id ?? null,
    ine: sections.has("identite") || sections.has("scolarite") ? raw.ine ?? null : null,
    nom: raw.nom,
    prenom: raw.prenom,
    classe: sections.has("scolarite") || sections.has("identite") ? raw.classe ?? null : null,
    regime: sections.has("scolarite") ? raw.regime ?? null : null,
    status: raw.status ?? null,
  };

  // Accueil : sections vides → identité minimale pour la liste (nom/prénom/classe), jamais foyer.
  if (sections.size === 0 && roles.includes("accueil")) {
    return {
      id: raw.id ?? null,
      ine: null,
      nom: raw.nom,
      prenom: raw.prenom,
      classe: raw.classe ?? null,
      regime: null,
      status: raw.status ?? null,
    };
  }

  if (!sections.has("famille")) {
    return base;
  }

  const withFamille: ElevePublicFact = { ...base };
  for (const k of FAMILLE_KEYS) {
    const v = raw[k];
    if (v != null && String(v).trim()) withFamille[k] = String(v).trim();
  }
  return withFamille;
}

export function explainOccupancyTag(tag: OccupancyTag): string {
  switch (tag) {
    case "en_sortie":
      return "En sortie scolaire (participation voyage) — ce n’est pas une absence bulletin.";
    case "en_stage":
      return "En période de stage / PFMP.";
    case "absent_vs":
      return "Absent au registre vie scolaire (accueil, appel, justif…).";
    case "internat":
      return "Présence / signal internat (couverture souvent partielle).";
    case "en_cours":
      return "Présumé en cours / dans les murs (aucun signal ailleurs).";
    case "inconnu":
      return "Présence inconnue (données insuffisantes).";
    default:
      return "Tag de présence non documenté.";
  }
}
