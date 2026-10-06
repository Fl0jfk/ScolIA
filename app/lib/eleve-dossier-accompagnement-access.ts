/**
 * Visibilité des dispositifs d’accompagnement (PAP / PAI / PPS / GEVASCO) — art. 9 RGPD.
 * Les professeurs « purs » peuvent voir la liste ouverte des élèves sans ces métadonnées.
 */

import { studentInAssignedClasses } from "@/app/lib/class-allocation-teachers";
import {
  isProfesseurScopedDossierViewer,
  PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES,
} from "@/app/lib/eleve-dossier-scope";

export type DossierAccompagnementViewer = {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
};

export type AccompagnementEleveContext = {
  eleveClasse?: string | null;
  assignedClasses?: string[];
};

/**
 * Interrupteur futur : à `true`, les profs voient PAP/PAI/PPS/GEVASCO uniquement
 * pour les élèves de leurs classes / groupes affectés (pas pour toute l’établissement).
 * Tant que l’affectation roster n’est pas fiable, laisser à `false`.
 */
export { PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES } from "@/app/lib/eleve-dossier-scope";

/** Suffixe cache liste dossiers : évite de servir une réponse « hub » à un prof. */
export function eleveDossierAccompagnementCacheScopeSuffix(
  viewer: DossierAccompagnementViewer,
): string {
  if (!isProfesseurScopedDossierViewer(viewer)) return "accomp:full";
  if (!PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES) return "accomp:none";
  return "accomp:prof-classes";
}

/** La liste dossiers peut-elle charger / renvoyer des champs d’accompagnement ? */
export function viewerMayLoadEleveAccompagnementListMetadata(
  viewer: DossierAccompagnementViewer,
  professeurSeesOwnClassesOverride?: boolean,
): boolean {
  if (!isProfesseurScopedDossierViewer(viewer)) return true;
  const flag = professeurSeesOwnClassesOverride ?? PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES;
  return flag;
}

/**
 * Métadonnées / fichiers d’accompagnement pour un élève donné.
 * `professeurSeesOwnClassesOverride` : tests unitaires uniquement.
 */
export function viewerMayReceiveEleveAccompagnementMetadata(
  viewer: DossierAccompagnementViewer,
  ctx?: AccompagnementEleveContext,
  professeurSeesOwnClassesOverride?: boolean,
): boolean {
  if (!isProfesseurScopedDossierViewer(viewer)) return true;
  const flag = professeurSeesOwnClassesOverride ?? PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES;
  if (!flag) return false;
  const assigned = ctx?.assignedClasses ?? [];
  if (!assigned.length) return false;
  return studentInAssignedClasses(ctx?.eleveClasse ?? undefined, assigned);
}
