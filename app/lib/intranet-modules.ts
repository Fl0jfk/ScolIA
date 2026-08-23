/**
 * ═══════════════════════════════════════════════════════════════════
 *  CATALOGUE UNIQUE DES MODULES INTRANET
 *  Ajouter / modifier un module ICI uniquement :
 *  - tuile dashboard (champ `dashboard`)
 *  - rôles autorisés (`allowedRoles`)
 *  - routes protégées par le middleware (`pathPrefixes`, si module interne)
 * ═══════════════════════════════════════════════════════════════════
 */

import { hasGlobalAdminRole, hasMasterRole, hasRole, isEleveOnlyRoleSet, normRole } from "./intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS, intranetRolesExceptParent, rolesFromUserLike } from "./intranet-roles";
import { RGPD_MODULE_ROLES } from "./rgpd-access";

const DIRECTIONS = [...INTRANET_DIRECTION_SLUGS];
const ROLES_EXCEPT_PARENT = intranetRolesExceptParent();

export type DashboardTileVariant =
  | "default"
  | "travels"
  | "prof-room"
  | "agent-ia"
  | "absences"
  | "personnel-ogec"
  | "internat"
  | "week-sheet"
  | "photocopies-couleur"
  | "toolbox";

export type DashboardCategory = {
  id: number;
  moduleId: string;
  name: string;
  link: string;
  img: string;
  description?: string;
  allowedRoles: string[];
  external?: boolean;
  orgAdminOnly?: boolean;
  variant?: DashboardTileVariant;
};

export type ExternalQuickLink = {
  id: string;
  name: string;
  link: string;
  img: string;
  allowedRoles: string[];
};

type IntranetModule = {
  id: string;
  allowedRoles: string[];
  orgAdminOnly?: boolean;
  /** Routes page + API. Omis pour les liens externes (tuile seulement). */
  pathPrefixes?: string[];
  excludePrefixes?: string[];
  /** Tuile sur le dashboard. Omis si module API-only (ex. règle complémentaire). */
  dashboard?: Omit<DashboardCategory, "allowedRoles" | "orgAdminOnly" | "moduleId">;
};

/** Accessible à tout utilisateur connecté (hors contrôle module). */
const INTRANET_ALWAYS_ALLOWED_PREFIXES = [
  "/dashboard",
  "/api/app/context",
  "/api/teams-chat",
  "/api/tenant/public",
  "/api/tenant/diagnostics",
  "/onboarding",
  "/configuration-en-cours",
  "/abonnement-suspendu",
  "/api/onboarding/status",
  "/api/billing/tenant/status",
];

/** Profil élève : accès minimal (dashboard + bulle bien-être). */
const INTRANET_ELEVE_ALLOWED_PREFIXES = [
  "/dashboard",
  "/bien-etre",
  "/api/dashboard",
  "/api/bien-etre",
  "/api/app/context",
  "/api/tenant/public",
  "/sign-in",
  "/sign-up",
  "/sign-out",
  "/sso-callback",
];

/** Réservé au profil Master (invisible, config plateforme). */
const INTRANET_PLATFORM_MASTER_PREFIXES = [
  "/platform/setup",
  "/api/platform",
  "/plateforme",
];

export const INTRANET_MODULES: IntranetModule[] = [
  {
    id: "documents",
    pathPrefixes: ["/documents", "/api/documents"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "professeur",
      "maintenance",
    ],
    dashboard: {
      id: 1,
      name: "Cloud personnel",
      img: "",
      link: "/documents",
      external: false,
    },
  },
  {
    id: "faire-demande",
    pathPrefixes: ["/faire-une-demande", "/demande-parents", "/demande/merci"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "professeur",
      "maintenance",
    ],
  },
  {
    id: "travels",
    pathPrefixes: ["/travels", "/api/travels"],
    excludePrefixes: ["/api/travels/ingest-from-email", "/api/travels/poll-email"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "professeur",
    ],
    dashboard: {
      id: 4,
      name: "Sortie scolaire",
      img: "",
      link: "/travels",
      external: false,
      variant: "travels",
    },
  },
  {
    id: "qrcreator",
    pathPrefixes: ["/qrcreator"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
  },
  {
    id: "toolbox",
    pathPrefixes: ["/toolbox", "/api/toolbox"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
    orgAdminOnly: false,
    dashboard: {
      id: 5,
      name: "Boîte à outils",
      img: "",
      link: "/toolbox",
      external: false,
      variant: "toolbox",
      description: "QR code et photocopies couleur.",
    },
  },
  {
    id: "prof-room",
    pathPrefixes: ["/prof-room", "/api/reservation-rooms"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "education",
      "cpe",
      "professeur",
      "maintenance",
    ],
    dashboard: {
      id: 8,
      name: "Réservation de salle",
      img: "",
      link: "/prof-room",
      external: false,
      variant: "prof-room",
    },
  },
  {
    id: "domain-planning",
    pathPrefixes: ["/domain-planning", "/api/domain-planning"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "education",
      "cpe",
      "professeur",
      "infirmerie",
      "psychologue",
      "maintenance",
    ],
    dashboard: {
      id: 25,
      name: "Enseignements transversaux",
      img: "",
      link: "/domain-planning",
      external: false,
      description: "Positionnements EVARS, séances et validation des intervenants — comme une réservation de salle.",
    },
  },
  {
    id: "agent-ia-ocr",
    pathPrefixes: ["/agentIAOCR", "/api/agentIAOCR", "/api/eleves", "/api/mef-secteurs", "/api/enseignants"],
    allowedRoles: ["administratif", "comptabilite", "education", "cpe", ...DIRECTIONS],
    dashboard: {
      id: 10,
      name: "Ajout de documents IA",
      img: "",
      link: "/agentIAOCR",
      external: false,
      variant: "agent-ia",
    },
  },
  {
    id: "eleve-dossier",
    pathPrefixes: [
      "/eleves/dossiers",
      "/eleves/dossier",
      "/api/eleves",
      "/preinscription",
      "/api/public/preinscriptions",
    ],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "professeur",
      "infirmerie",
    ],
    dashboard: {
      id: 42,
      name: "Dossiers élèves",
      img: "",
      link: "/eleves/dossiers",
      external: false,
      description: "Fiche élève unique — famille, scolarité, documents cloisonnés.",
    },
  },
  {
    id: "channels",
    pathPrefixes: ["/channels", "/api/channels"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
    dashboard: {
      id: 11,
      name: "Salons",
      img: "",
      link: "/channels",
      external: false,
    },
  },
  {
    id: "absences",
    pathPrefixes: ["/absences", "/api/absences", "/calendrierAbsProfs"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "education",
      "cpe",
      "comptabilite",
      "professeur",
      "maintenance",
    ],
    // Tuile dashboard retirée : module absorbé dans RH (`/rh?tab=absences`).
  },
  {
    id: "organigramme",
    pathPrefixes: ["/organigramme", "/api/organigramme"],
    allowedRoles: [...ROLES_EXCEPT_PARENT].filter((r) => r !== "eleve"),
    dashboard: {
      id: 14,
      name: "Annuaire de l'établissement",
      img: "",
      link: "/organigramme",
      external: false,
      description: "Qui fait quoi — organisation, contacts et missions.",
    },
  },
  {
    id: "evenements",
    pathPrefixes: ["/etablissement/evenements"],
    allowedRoles: [...ROLES_EXCEPT_PARENT].filter((r) => r !== "eleve"),
    dashboard: {
      id: 30,
      name: "Événements",
      img: "",
      link: "/etablissement/evenements",
      external: false,
      description: "Portes ouvertes, rentrée digitale (dont fournitures) et Secret Santa.",
    },
  },
  {
    id: "communication",
    pathPrefixes: [
      "/etablissement/communication",
      "/api/document-templates",
      "/api/posters",
      "/api/posters/drafts",
    ],
    allowedRoles: [],
    orgAdminOnly: true,
    dashboard: {
      id: 32,
      name: "Communication",
      img: "",
      link: "/etablissement/communication",
      external: false,
      description: "Création : documents familles + affiches, tarifs et actus site (option vitrine).",
    },
  },
  {
    id: "identite",
    allowedRoles: [],
    orgAdminOnly: true,
    // Tuile retirée : absorbée dans « Paramètres » (`admin-settings`).
  },
  {
    id: "requests-staff",
    pathPrefixes: ["/requests", "/mes-demandes", "/api/requests"],
    excludePrefixes: ["/api/requests/create", "/api/requests/confirm"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "professeur",
      "maintenance",
    ],
    dashboard: {
      id: 3,
      name: "Demandes",
      img: "",
      link: "/requests",
      external: false,
    },
  },
  {
    id: "chatbot-knowledge",
    pathPrefixes: ["/chatbot-knowledge", "/api/chatbot/ingest"],
    allowedRoles: ["administratif"],
    dashboard: {
      id: 17,
      name: "Brain AI (training engine)",
      img: "",
      link: "/chatbot-knowledge",
      external: false,
    },
  },
  {
    id: "photocopies-couleur",
    pathPrefixes: ["/photocopies-couleur", "/api/photocopies-couleur"],
    allowedRoles: [...DIRECTIONS, "administratif", "professeur"],
    // Tuile dashboard retirée : accès via Boîte à outils (+ alertes direction si file d’attente).
    // Route /photocopies-couleur conservée.
  },
  {
    id: "demandes-hse",
    pathPrefixes: ["/demandes-hse", "/api/demandes-hse"],
    allowedRoles: [...DIRECTIONS, "professeur"],
    // Tuile dashboard retirée : module absorbé dans RH (`/rh?tab=hse`).
    // Visibilité métier : soi (prof) + direction établissement uniquement.
  },
  {
    id: "conformite-rgpd",
    pathPrefixes: ["/conformite-rgpd", "/api/rgpd"],
    allowedRoles: [...RGPD_MODULE_ROLES],
    dashboard: {
      id: 29,
      name: "Conformité RGPD",
      img: "",
      link: "/conformite-rgpd",
      external: false,
    },
  },
  {
    id: "admin-settings",
    pathPrefixes: ["/parametres", "/api/settings", "/membres", "/api/members"],
    allowedRoles: [],
    orgAdminOnly: true,
    dashboard: {
      id: 21,
      name: "Paramètres",
      img: "",
      link: "/parametres",
      external: false,
      description: "Utilisateurs, établissement, liste des élèves et réglages globaux du SaaS.",
    },
  },
  {
    id: "rh",
    pathPrefixes: ["/rh", "/personnel", "/api/personnel", "/api/rh"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "maintenance",
      "professeur",
    ],
    dashboard: {
      id: 23,
      name: "Module RH",
      img: "",
      link: "/rh",
      external: false,
      variant: "personnel-ogec",
    },
  },
  {
    id: "mon-planning",
    pathPrefixes: ["/mon-planning", "/edt-classe", "/edt-etablissement", "/api/edt"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "comptabilite",
      "education",
      "cpe",
      "maintenance",
      "professeur",
    ],
    dashboard: {
      id: 32,
      name: "Mon planning",
      img: "",
      link: "/mon-planning",
      external: false,
      description: "Emploi du temps semaine — A/B, missions, quota.",
    },
  },
  {
    id: "pilotage-eleves",
    pathPrefixes: ["/pilotage-eleves", "/api/pilotage-eleves"],
    allowedRoles: ["direction_ecole", "direction_college", "direction_lycee", "administratif"],
    dashboard: {
      id: 41,
      name: "Pilotage élèves",
      img: "",
      link: "/pilotage-eleves",
      external: false,
      description: "Aide documentaire au conseil de classe — dossiers classés, pas la vie scolaire live",
    },
  },
  {
    id: "internat",
    pathPrefixes: ["/gestion-internat", "/api/internat"],
    allowedRoles: [...DIRECTIONS, "administratif", "education", "cpe"],
    dashboard: {
      id: 24,
      name: "Internat",
      img: "",
      link: "/gestion-internat",
      external: false,
      variant: "internat",
    },
  },
  {
    id: "stages",
    pathPrefixes: ["/stages", "/api/stages"],
    excludePrefixes: ["/stages/eleve", "/stages/signer", "/stages/candidater", "/api/stages/public"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "professeur",
      "education",
      "cpe",
      "parent",
    ],
    dashboard: {
      id: 28,
      name: "Stages & conventions",
      img: "",
      link: "/stages",
      external: false,
      description: "Offres parents, préconventions, signatures PFMP et jobs d'été",
    },
  },
  {
    id: "certificates",
    pathPrefixes: ["/certificates", "/api/certificates"],
    excludePrefixes: ["/certificates/verify", "/api/certificates/verify"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "professeur",
    ],
    dashboard: {
      id: 29,
      name: "Parcours & certificats",
      img: "",
      link: "/certificates",
      external: false,
      description: "Certificats de parcours, lignes personnalisées, signatures et PDF",
    },
  },
  {
    id: "covoiturage",
    pathPrefixes: ["/covoiturage", "/api/covoiturage"],
    allowedRoles: ["parent"],
    dashboard: {
      id: 26,
      name: "Covoiturage",
      img: "",
      link: "/covoiturage",
      external: false,
      variant: "default",
    },
  },
  {
    id: "assistance",
    pathPrefixes: ["/assistance", "/api/assistance"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
    dashboard: {
      id: 27,
      name: "Assistance",
      img: "",
      link: "/assistance",
      external: false,
      description: "Signaler un problème technique",
    },
  },
  {
    id: "notes",
    pathPrefixes: ["/notes"],
    allowedRoles: [...DIRECTIONS, "professeur"],
    dashboard: {
      id: 51,
      name: "Notes & bulletins",
      img: "",
      link: "/notes/espace",
      external: false,
      description: "Notes, évaluations et bulletins (module P4 — bientôt).",
    },
  },
  {
    id: "sante",
    pathPrefixes: ["/sante"],
    allowedRoles: ["infirmerie", "psychologue"],
    dashboard: {
      id: 52,
      name: "Espace santé",
      img: "",
      link: "/sante/espace",
      external: false,
      description: "Infirmerie, PAP et suivi santé des élèves.",
    },
  },
  {
    id: "bien-etre-referent",
    pathPrefixes: ["/bien-etre/referent", "/bien-etre/config", "/api/bien-etre/config", "/api/bien-etre/signalements"],
    allowedRoles: [...DIRECTIONS, "administratif", "education", "cpe", "infirmerie", "psychologue"],
    // Tuile dashboard masquée pour l’instant (module pas encore prêt).
  },
  {
    id: "dashboard-week-sheet",
    pathPrefixes: [
      "/api/dashboard/week-sheet",
      "/api/dashboard/links",
      "/api/dashboard/signals",
      "/api/weather",
    ],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
    // Plus de tuile dashboard : contenu exposé via le ticker « Actualité du jour ».
  },
  {
    id: "pillar-administratif",
    pathPrefixes: ["/administratif"],
    allowedRoles: [...DIRECTIONS, "administratif", "admin", "professeur"],
  },
  {
    id: "pillar-services",
    pathPrefixes: ["/services"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "professeur",
      "cpe",
      "education",
      "comptabilite",
      "maintenance",
      "admin",
    ],
  },
  {
    id: "pillar-vie-scolaire",
    pathPrefixes: ["/vie-scolaire"],
    allowedRoles: [...DIRECTIONS, "cpe", "education"],
  },
  {
    id: "pillar-compta-rh",
    pathPrefixes: ["/compta-rh"],
    allowedRoles: [...DIRECTIONS, "comptabilite", "administratif", "admin", "maintenance"],
  },
  {
    id: "scolia-ai",
    pathPrefixes: ["/scolia-ai"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
  },
  {
    id: "legacy-hub-redirects",
    pathPrefixes: ["/eleves", "/etablissement", "/rh"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
  },
];

/** Raccourcis externes sous le slider (mêmes règles de rôles, pas de route interne). */
const INTRANET_EXTERNAL_QUICK_LINKS: ExternalQuickLink[] = [
  {
    id: "ecole-directe",
    name: "École Directe",
    img: "",
    link: "https://www.ecoledirecte.com/login?cameFrom=%2FAccueil",
    allowedRoles: [
      "direction_college",
      "administratif",
      "professeur",
      "direction_ecole",
      "direction_lycee",
      "maintenance",
      "comptabilite",
      "infirmerie",
      "education",
      "cpe",
    ],
  },
  {
    id: "zeendoc",
    name: "ZeenDoc",
    img: "",
    link: "https://armoires.zeendoc.com/_Login/Login.php",
    allowedRoles: [
      "administratif",
      "comptabilite",
      "direction_college",
      "direction_ecole",
      "direction_lycee",
    ],
  },
  {
    id: "arena",
    name: "Arena Ac-Normandie",
    img: "",
    link: "https://arena.ac-normandie.fr/arena/",
    allowedRoles: ["administratif", "direction_college", "direction_ecole", "direction_lycee"],
  },
];

export function getDashboardCategories(): DashboardCategory[] {
  return INTRANET_MODULES.filter((m) => m.dashboard)
    .map((m) => ({
      ...m.dashboard!,
      moduleId: m.id,
      allowedRoles: m.allowedRoles,
      orgAdminOnly: m.orgAdminOnly,
    }))
    .sort((a, b) => a.id - b.id);
}

function getExternalQuickLinks(): ExternalQuickLink[] {
  return INTRANET_EXTERNAL_QUICK_LINKS;
}

function normalizePathname(pathname: string): string {
  const p = pathname.split("?")[0] || "/";
  if (p.length > 1 && p.endsWith("/")) return p.slice(0, -1);
  return p;
}

function isExcluded(pathname: string, excludePrefixes?: string[]): boolean {
  if (!excludePrefixes?.length) return false;
  return excludePrefixes.some((ex) => pathname === ex || pathname.startsWith(`${ex}/`));
}

function moduleMatchesPath(module: IntranetModule, pathname: string): boolean {
  if (!module.pathPrefixes?.length) return false;
  if (isExcluded(pathname, module.excludePrefixes)) return false;
  return module.pathPrefixes.some(
    (prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`),
  );
}

function findMatchingModules(pathname: string): IntranetModule[] {
  const normalized = normalizePathname(pathname);
  return INTRANET_MODULES.filter((m) => moduleMatchesPath(m, normalized));
}

export function rolesAllowModule(
  roles: string[],
  module: IntranetModule,
  isOrgAdmin: boolean,
): boolean {
  if (hasMasterRole(roles)) return true;
  if (module.id === "pilotage-eleves") {
    const allowed = new Set(
      ["direction_ecole", "direction_college", "direction_lycee", "administratif"].map(normRole),
    );
    return roles.some((r) => allowed.has(normRole(r)));
  }
  if (module.orgAdminOnly) return isOrgAdmin;
  if (hasGlobalAdminRole(roles)) return true;
  if (!module.allowedRoles.length) return false;
  return module.allowedRoles.some((r) => hasRole(roles, r));
}

export function canAccessIntranetPath(
  pathname: string,
  roles: string[],
  isOrgAdmin: boolean,
): boolean {
  const normalized = normalizePathname(pathname);

  if (
    INTRANET_PLATFORM_MASTER_PREFIXES.some(
      (p) => normalized === p || normalized.startsWith(`${p}/`),
    )
  ) {
    return hasMasterRole(roles);
  }

  if (isEleveOnlyRoleSet(roles)) {
    return INTRANET_ELEVE_ALLOWED_PREFIXES.some(
      (p) => normalized === p || normalized.startsWith(`${p}/`),
    );
  }

  if (
    INTRANET_ALWAYS_ALLOWED_PREFIXES.some(
      (p) => normalized === p || normalized.startsWith(`${p}/`),
    )
  ) {
    return true;
  }

  const modules = findMatchingModules(normalized);
  if (modules.length === 0) return false;

  return modules.some((m) => rolesAllowModule(roles, m, isOrgAdmin));
}

export function getIntranetModuleById(moduleId: string): IntranetModule | undefined {
  return INTRANET_MODULES.find((m) => m.id === moduleId);
}

/** Module intranet correspondant à un chemin (préfixe le plus spécifique). */
export function resolveModuleIdFromPath(pathname: string): string | null {
  const normalized = normalizePathname(pathname);
  const matches = findMatchingModules(normalized);
  if (!matches.length) return null;

  let best: IntranetModule = matches[0]!;
  let bestLen = 0;
  for (const m of matches) {
    for (const prefix of m.pathPrefixes ?? []) {
      if (normalized === prefix || normalized.startsWith(`${prefix}/`)) {
        if (prefix.length > bestLen) {
          bestLen = prefix.length;
          best = m;
        }
      }
    }
  }
  return best.id;
}

export function isOrgAdminFromSession(
  orgRole: string | null | undefined,
  publicMetadata: Record<string, unknown> | undefined,
): boolean {
  const roleArr = rolesFromUserLike({ publicMetadata });
  return (
    orgRole === "org:admin" ||
    roleArr.includes("admin") ||
    roleArr.includes("master") ||
    publicMetadata?.org_admin === true ||
    publicMetadata?.platform_admin === true
  );
}
