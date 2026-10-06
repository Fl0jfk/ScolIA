/**
 * Visibilité des dispositifs d’accompagnement (PAP / PAI / PPS / GEVASCO) — art. 9 RGPD.
 * Les professeurs « purs » peuvent voir la liste ouverte des élèves sans ces métadonnées.
 */

import { studentInAssignedClasses } from "@/app/lib/class-allocation-teachers";
import { hasRole } from "@/app/lib/intranet-role-utils";
import {
  isProfesseurScopedDossierViewer,
  PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES,
} from "@/app/lib/eleve-dossier-scope";
import { ACCOMPAGNEMENT_KINDS, type AccompagnementKind } from "@/app/lib/eleve-pap";

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

/** Prof « pur » + rôle internat : seul le PAI (sécurité internat), pas le tiroir santé complet. */
export const PROF_INTERNAT_ACCOMPAGNEMENT_KINDS: readonly AccompagnementKind[] = ["pai"];

export function isProfesseurInternatPaiOnlyViewer(
  viewer: DossierAccompagnementViewer,
): boolean {
  if (!isProfesseurScopedDossierViewer(viewer)) return false;
  return hasRole(viewer.roles, "internat");
}

/**
 * Restriction RGPD « professeur sans hub dossier » (prof seul ou prof + internat).
 */
export function isTeacherOnlyAccompagnementRestricted(
  viewer: DossierAccompagnementViewer,
): boolean {
  return isProfesseurScopedDossierViewer(viewer);
}

/** Suffixe cache liste dossiers : évite de servir une réponse « hub » à un prof. */
export function eleveDossierAccompagnementCacheScopeSuffix(
  viewer: DossierAccompagnementViewer,
): string {
  if (!isTeacherOnlyAccompagnementRestricted(viewer)) return "accomp:full";
  if (isProfesseurInternatPaiOnlyViewer(viewer)) return "accomp:pai";
  if (!PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES) return "accomp:none";
  return "accomp:prof-classes";
}

/** La liste dossiers peut-elle charger / renvoyer des champs d’accompagnement ? */
export function viewerMayLoadEleveAccompagnementListMetadata(
  viewer: DossierAccompagnementViewer,
  professeurSeesOwnClassesOverride?: boolean,
): boolean {
  if (!isTeacherOnlyAccompagnementRestricted(viewer)) return true;
  if (isProfesseurInternatPaiOnlyViewer(viewer)) return true;
  const flag = professeurSeesOwnClassesOverride ?? PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES;
  return flag;
}

/**
 * Métadonnées / fichiers d’accompagnement pour un élève donné (au moins un dispositif visible).
 * `professeurSeesOwnClassesOverride` : tests unitaires uniquement.
 */
export function viewerMayReceiveEleveAccompagnementMetadata(
  viewer: DossierAccompagnementViewer,
  ctx?: AccompagnementEleveContext,
  professeurSeesOwnClassesOverride?: boolean,
): boolean {
  if (!isTeacherOnlyAccompagnementRestricted(viewer)) return true;
  if (isProfesseurInternatPaiOnlyViewer(viewer)) return true;
  const flag = professeurSeesOwnClassesOverride ?? PROFESSEUR_SEES_ACCOMPAGNEMENTS_OWN_CLASSES;
  if (!flag) return false;
  const assigned = ctx?.assignedClasses ?? [];
  if (!assigned.length) return false;
  return studentInAssignedClasses(ctx?.eleveClasse ?? undefined, assigned);
}

/** Un type précis (PAP / PAI / …) pour ce viewer et cet élève. */
export function viewerMayReceiveAccompagnementKind(
  viewer: DossierAccompagnementViewer,
  kind: AccompagnementKind,
  ctx?: AccompagnementEleveContext,
  professeurSeesOwnClassesOverride?: boolean,
): boolean {
  if (!isTeacherOnlyAccompagnementRestricted(viewer)) return true;
  if (isProfesseurInternatPaiOnlyViewer(viewer)) {
    return PROF_INTERNAT_ACCOMPAGNEMENT_KINDS.includes(kind);
  }
  if (
    !viewerMayReceiveEleveAccompagnementMetadata(
      viewer,
      ctx,
      professeurSeesOwnClassesOverride,
    )
  ) {
    return false;
  }
  return true;
}

export type BrainAccompagnementItem = {
  kind: AccompagnementKind;
  documentId: string;
};

/** Filtre PAP/PAI/PPS/GEVASCO pour l’assistant (brain) — sans accès BDD. */
export function brainAccompagnementExposure(
  viewer: DossierAccompagnementViewer,
  eleveClasse: string | null | undefined,
  rawItems: readonly BrainAccompagnementItem[],
  assignedClasses?: string[],
): { kinds: AccompagnementKind[]; items: BrainAccompagnementItem[] } {
  if (!viewerMayLoadEleveAccompagnementListMetadata(viewer)) {
    return { kinds: [], items: [] };
  }
  const ctx: AccompagnementEleveContext = {
    eleveClasse,
    assignedClasses,
  };
  const items = rawItems.filter((i) =>
    viewerMayReceiveAccompagnementKind(viewer, i.kind, ctx),
  );
  const kindOrder = ACCOMPAGNEMENT_KINDS.map((k) => k.kind);
  const kinds = kindOrder.filter((k) => items.some((i) => i.kind === k));
  return { kinds, items: [...items] };
}

export function dossierViewerFromBrainCtx(ctx: {
  roles: string[];
  isOrgAdmin: boolean;
}): DossierAccompagnementViewer {
  return { roles: ctx.roles, orgAdmin: ctx.isOrgAdmin, platformAdmin: false };
}

/** Alertes tableau de bord (PAP/PAI/PPS/GEVASCO) — prof pur exclu si interrupteur false. */
export function filterAccompagnementAlertsForViewer<
  T extends { classe: string | null; kind?: AccompagnementKind },
>(
  viewer: DossierAccompagnementViewer,
  alerts: readonly T[],
  assignedClasses?: string[],
): T[] {
  if (!viewerMayReceiveAccompagnementDashboardAlerts(viewer)) {
    return [];
  }
  if (!isTeacherOnlyAccompagnementRestricted(viewer)) {
    return [...alerts];
  }
  return alerts.filter((a) => {
    if (
      a.kind &&
      !viewerMayReceiveAccompagnementKind(viewer, a.kind, {
        eleveClasse: a.classe,
        assignedClasses,
      })
    ) {
      return false;
    }
    return viewerMayReceiveEleveAccompagnementMetadata(viewer, {
      eleveClasse: a.classe,
      assignedClasses,
    });
  });
}

/** Le professeur « pur » doit-il voir le flux d’alertes accompagnement sur le dashboard ? */
export function viewerMayReceiveAccompagnementDashboardAlerts(
  viewer: DossierAccompagnementViewer,
): boolean {
  if (!isTeacherOnlyAccompagnementRestricted(viewer)) return true;
  return viewerMayLoadEleveAccompagnementListMetadata(viewer);
}
