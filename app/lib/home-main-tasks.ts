import { moduleHref, MODULE_EMOJI } from "@/app/lib/pillar-module-routes";
import { hasRole } from "@/app/lib/intranet-role-utils";

export type HomeMainTask = {
  id: string;
  label: string;
  detail: string;
  href: string;
  emoji: string;
  moduleId: string;
};

type ResolveOpts = {
  roles: string[];
  accessibleModuleIds?: Set<string> | null;
  orgAdmin?: boolean;
};

function canAccess(opts: ResolveOpts, moduleId: string): boolean {
  if (opts.orgAdmin) return true;
  if (!opts.accessibleModuleIds) return true;
  return opts.accessibleModuleIds.has(moduleId);
}

/**
 * Raccourcis vitaux d’accueil selon les rôles (pas les notifs).
 * Une seule entrée par module, ordre = priorité métier.
 */
export function resolveHomeMainTasks(opts: ResolveOpts): HomeMainTask[] {
  const roles = opts.roles;
  const out: HomeMainTask[] = [];
  const seen = new Set<string>();

  const push = (task: HomeMainTask) => {
    if (seen.has(task.moduleId)) return;
    if (!canAccess(opts, task.moduleId)) return;
    seen.add(task.moduleId);
    out.push(task);
  };

  // Accueil → déclaration d’absences en premier
  if (hasRole(roles, "accueil")) {
    push({
      id: "task-accueil-absences",
      moduleId: "accueil-absences",
      label: "Absences accueil",
      detail: "Déclarer au standard",
      href: `${moduleHref("accueil-absences")}?tab=declarer`,
      emoji: MODULE_EMOJI["accueil-absences"] || "☎️",
    });
  }

  // Internat → appel du soir
  if (hasRole(roles, "internat")) {
    push({
      id: "task-internat-appel",
      moduleId: "internat",
      label: "Internat",
      detail: "Appel du soir",
      href: `${moduleHref("internat")}?tab=appel`,
      emoji: MODULE_EMOJI.internat || "🌙",
    });
  }

  // Surveillant → appels de présence
  if (hasRole(roles, "surveillant")) {
    push({
      id: "task-vs-appels",
      moduleId: "vs-appels",
      label: "Appels",
      detail: "Présence en classe",
      href: moduleHref("vs-appels"),
      emoji: MODULE_EMOJI["vs-appels"] || "✅",
    });
    push({
      id: "task-accueil-consult",
      moduleId: "accueil-absences",
      label: "Absences élèves",
      detail: "Saisies du jour",
      href: `${moduleHref("accueil-absences")}?tab=consulter`,
      emoji: MODULE_EMOJI["accueil-absences"] || "☎️",
    });
  }

  // Administratif / admin → dossiers élèves
  if (hasRole(roles, "administratif") || hasRole(roles, "admin") || opts.orgAdmin) {
    push({
      id: "task-eleve-dossier",
      moduleId: "eleve-dossier",
      label: "Dossiers élèves",
      detail: "Fiches & documents",
      href: moduleHref("eleve-dossier"),
      emoji: MODULE_EMOJI["eleve-dossier"] || "📁",
    });
  }

  // Direction → sorties / dossiers
  if (
    hasRole(roles, "direction_ecole") ||
    hasRole(roles, "direction_college") ||
    hasRole(roles, "direction_lycee") ||
    hasRole(roles, "direction")
  ) {
    push({
      id: "task-travels-dir",
      moduleId: "travels",
      label: "Sorties scolaires",
      detail: "Validations & suivi",
      href: moduleHref("travels"),
      emoji: MODULE_EMOJI.travels || "🚌",
    });
    push({
      id: "task-eleve-dossier-dir",
      moduleId: "eleve-dossier",
      label: "Dossiers élèves",
      detail: "Fiches & inscriptions",
      href: moduleHref("eleve-dossier"),
      emoji: MODULE_EMOJI["eleve-dossier"] || "📁",
    });
  }

  // Professeur → planning (notes plus tard)
  if (hasRole(roles, "professeur")) {
    push({
      id: "task-mon-planning",
      moduleId: "mon-planning",
      label: "Mon planning",
      detail: "Emploi du temps",
      href: moduleHref("mon-planning"),
      emoji: MODULE_EMOJI["mon-planning"] || "🗓️",
    });
    push({
      id: "task-prof-room",
      moduleId: "prof-room",
      label: "Salles",
      detail: "Réserver une salle",
      href: moduleHref("prof-room"),
      emoji: MODULE_EMOJI["prof-room"] || "🚪",
    });
  }

  // Comptabilité / RH → absences personnel
  if (hasRole(roles, "comptabilite") || hasRole(roles, "rh")) {
    push({
      id: "task-absences-rh",
      moduleId: "absences",
      label: "Absences RH",
      detail: "Autorisations & calendrier",
      href: "/rh?tab=dashboard&section=absences",
      emoji: MODULE_EMOJI.absences || "🤒",
    });
  }

  // Vie scolaire générique
  if (hasRole(roles, "vie_scolaire") || hasRole(roles, "viescolaire")) {
    push({
      id: "task-vs-accueil",
      moduleId: "accueil-absences",
      label: "Vie scolaire",
      detail: "Absences & appels",
      href: moduleHref("accueil-absences"),
      emoji: MODULE_EMOJI["accueil-absences"] || "☎️",
    });
  }

  return out.slice(0, 6);
}
