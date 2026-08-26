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
import {
  customDefaultModulesForRole,
  hasCustomRoleDefaults,
} from "./module-access-defaults";

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

export type IntranetModule = {
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
  /** Compte : MDP / e-mail / events — sinon « Accès refusé à ce module » au premier login. */
  "/api/account",
  "/api/me/module-access",
  /** Chrome dashboard (météo, actualité) — hors matrice « Droits modules ». */
  "/api/weather",
  "/api/dashboard",
  "/onboarding",
  "/configuration-en-cours",
  "/abonnement-suspendu",
  "/api/onboarding/status",
  "/api/billing/tenant/status",
  /** Portail famille / app mobile parents — hors modules intranet staff. */
  "/api/famille",
  "/famille",
  "/api/eleve",
  "/api/mobile",
  "/app-mobile",
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
      "surveillant",
      "cpe",
      "professeur",
      "maintenance",
      "infirmerie",
      "psychologue",
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
      "surveillant",
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
      "surveillant",
      "cpe",
      "professeur",
      "infirmerie",
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
      description: "QR code, photocopies et outils activables.",
    },
  },
  {
    id: "prof-room",
    pathPrefixes: ["/prof-room", "/api/reservation-rooms"],
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "surveillant",
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
      "surveillant",
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
    allowedRoles: ["administratif", "comptabilite", "surveillant", "cpe", ...DIRECTIONS],
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
      "surveillant",
      "cpe",
      "professeur",
      "infirmerie",
      "psychologue",
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
      "surveillant",
      "cpe",
      "comptabilite",
      "professeur",
      "maintenance",
      "infirmerie",
      "psychologue",
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
    allowedRoles: [...DIRECTIONS, "administratif", "admin"],
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
      "surveillant",
      "cpe",
      "professeur",
      "maintenance",
      "infirmerie",
      "psychologue",
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
    pathPrefixes: [
      "/parametres",
      "/api/settings",
      "/api/admin/auth",
      "/membres",
      "/api/members",
      "/api/nomenclature",
    ],
    allowedRoles: [...DIRECTIONS, "admin"],
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
      "surveillant",
      "cpe",
      "maintenance",
      "professeur",
      "infirmerie",
      "psychologue",
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
      "surveillant",
      "cpe",
      "maintenance",
      "professeur",
      "infirmerie",
      "psychologue",
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
    // Module masqué (UI + raccourcis) — API conservée pour OCR / jobs internes.
    allowedRoles: [],
  },
  {
    id: "internat",
    pathPrefixes: ["/gestion-internat", "/api/internat"],
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe", "infirmerie"],
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
    id: "vs-calendrier",
    pathPrefixes: ["/vie-scolaire/calendrier", "/api/vie-scolaire/calendrier"],
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe", "professeur"],
    dashboard: {
      id: 241,
      name: "Calendrier & EDT",
      img: "",
      link: "/vie-scolaire/calendrier",
      external: false,
      description: "Vacances scolaires et créneaux EDT classe",
    },
  },
  {
    id: "vs-appels",
    pathPrefixes: [
      "/vie-scolaire/presence",
      "/vie-scolaire/appels",
      "/api/vie-scolaire/appels",
    ],
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe", "professeur"],
    dashboard: {
      id: 242,
      name: "Appels & absences",
      img: "",
      link: "/vie-scolaire/presence",
      external: false,
      description: "Appel de classe, absents, justificatifs et relances CPE",
    },
  },
  {
    id: "vs-absences",
    pathPrefixes: ["/vie-scolaire/absences", "/api/vie-scolaire/absences"],
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe"],
    // Tuile absorbée dans « Appels & absences » (`vs-appels` → /vie-scolaire/presence).
  },
  {
    id: "vs-sanctions",
    pathPrefixes: ["/vie-scolaire/sanctions", "/api/vie-scolaire/sanctions"],
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe"],
    dashboard: {
      id: 244,
      name: "Sanctions",
      img: "",
      link: "/vie-scolaire/sanctions",
      external: false,
      description: "Catalogue court — avertissement, colle, blâme",
    },
  },
  {
    id: "vs-carnet",
    pathPrefixes: ["/vie-scolaire/carnet", "/api/vie-scolaire/carnet"],
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe", "professeur"],
    dashboard: {
      id: 245,
      name: "Carnet",
      img: "",
      link: "/vie-scolaire/carnet",
      external: false,
      description: "Correspondance établissement → famille + accusé",
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
      "surveillant",
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
      "cpe",
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
    allowedRoles: ["parent", ...DIRECTIONS, "administratif", "admin"],
    // Tuile dashboard retirée : outil activable via Boîte à outils (désactivé par défaut).
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
    pathPrefixes: ["/notes", "/api/notes"],
    allowedRoles: [...DIRECTIONS, "administratif", "admin", "cpe"],
    dashboard: {
      id: 51,
      name: "Notes & bulletins",
      img: "",
      link: "/notes/espace",
      external: false,
      description: "Référentiels, saisie, compétences LSU collège et bulletins PDF.",
    },
  },
  {
    id: "groupes-pedagogiques",
    pathPrefixes: ["/groupes-pedagogiques", "/api/groupes-pedagogiques"],
    allowedRoles: [...DIRECTIONS, "professeur", "administratif", "cpe", "surveillant", "admin"],
    dashboard: {
      id: 52,
      name: "Groupes pédagogiques",
      img: "",
      link: "/groupes-pedagogiques",
      external: false,
      description: "Options, LV2, groupes transversaux Notes + Vie scolaire.",
    },
  },
  {
    id: "facturation-familles",
    pathPrefixes: ["/facturation", "/api/facturation"],
    allowedRoles: [...DIRECTIONS, "comptabilite", "administratif", "admin"],
    // Tuile dashboard masquée : module pas encore prêt.
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
    allowedRoles: [...DIRECTIONS, "administratif", "surveillant", "cpe", "infirmerie", "psychologue"],
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
    id: "pillar-etablissement",
    pathPrefixes: ["/etablissement"],
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
      "surveillant",
      "comptabilite",
      "maintenance",
      "admin",
    ],
  },
  {
    id: "pillar-vie-scolaire",
    pathPrefixes: ["/vie-scolaire"],
    allowedRoles: [...DIRECTIONS, "cpe", "surveillant", "administratif", "admin", "professeur"],
  },
  {
    id: "pillar-compta-rh",
    pathPrefixes: ["/compta-rh"],
    allowedRoles: [...DIRECTIONS, "comptabilite", "administratif", "admin", "maintenance", "professeur"],
  },
  {
    id: "scolia-ai",
    pathPrefixes: ["/scolia-ai"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
  },
  {
    id: "legacy-hub-redirects",
    pathPrefixes: ["/eleves", "/rh"],
    allowedRoles: [...ROLES_EXCEPT_PARENT],
  },
  {
    id: "rh-paie-spec",
    pathPrefixes: ["/rh/paie", "/api/rh/paie-spec"],
    allowedRoles: [...DIRECTIONS, "comptabilite", "administratif", "admin"],
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
      "surveillant",
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

/**
 * Modules hors matrice Paramètres → Droits modules.
 * Ils ne doivent PAS être bloqués par une fiche byUser / byRole
 * (sinon météo / hubs legacy passent en 403 dès qu’on personnalise les cases).
 */
const MODULES_IGNORE_ACCESS_OVERRIDES = new Set([
  "dashboard-week-sheet",
  "legacy-hub-redirects",
  "scolia-ai",
  "pillar-administratif",
  "pillar-etablissement",
  "pillar-services",
  "pillar-vie-scolaire",
  "pillar-compta-rh",
  "rh-paie-spec",
]);

export function rolesAllowModule(
  roles: string[],
  module: IntranetModule,
  isOrgAdmin: boolean,
  access?: {
    byRole?: Record<string, { modules?: string[] }>;
    byUser?: Record<string, { modules?: string[] }>;
  } | null,
  userRef?: string | { userId?: string | null; businessUserId?: string | null } | null,
): boolean {
  if (hasMasterRole(roles)) return true;
  // Admin établissement (flag ou rôle) : tous les modules du tenant.
  if (isOrgAdmin || hasGlobalAdminRole(roles)) return true;
  if (module.orgAdminOnly) return false;
  // Pilotage élèves : masqué (pas d’accès rôle métier, hors orgAdmin).
  if (module.id === "pilotage-eleves") return false;

  // Hubs piliers : accessible dès qu’un module du pilier l’est (matrice / defaults).
  if (module.id.startsWith("pillar-")) {
    const childIds = PILLAR_HUB_CHILD_MODULES[module.id];
    if (childIds?.length) {
      return childIds.some((id) => {
        const child = getIntranetModuleById(id);
        return child ? rolesAllowModule(roles, child, isOrgAdmin, access, userRef) : false;
      });
    }
  }

  if (!module.allowedRoles.length) return false;

  // Infra / hors UI : uniquement le rôle métier natif (pas la fiche individuelle).
  if (MODULES_IGNORE_ACCESS_OVERRIDES.has(module.id)) {
    return module.allowedRoles.some((r) => hasRole(roles, r));
  }

  const byRole = access?.byRole;
  const byUser = access?.byUser;
  const candidateIds =
    typeof userRef === "string"
      ? [userRef.trim()].filter(Boolean)
      : [userRef?.userId, userRef?.businessUserId]
          .map((x) => (typeof x === "string" ? x.trim() : ""))
          .filter(Boolean);
  let userOv: { modules?: string[] } | undefined;
  if (byUser && candidateIds.length) {
    for (const id of candidateIds) {
      if (byUser[id]) {
        userOv = byUser[id];
        break;
      }
    }
  }
  if (userOv) {
    return (userOv.modules ?? []).includes(module.id);
  }

  const hasRoleOverrides = Boolean(byRole && Object.keys(byRole).length > 0);
  if (hasRoleOverrides) {
    for (const role of roles) {
      if (role === "admin") return true;
      const override = byRole![role];
      if (override) {
        if ((override.modules ?? []).includes(module.id)) return true;
        continue;
      }
      if (module.allowedRoles.some((r) => hasRole([role], r))) return true;
    }
    return false;
  }

  // Defaults métier explicites (ex. professeur) = source de vérité.
  const fromCustom = new Set<string>();
  let anyCustom = false;
  for (const role of roles) {
    if (!hasCustomRoleDefaults(role)) continue;
    anyCustom = true;
    for (const id of customDefaultModulesForRole(role) ?? []) fromCustom.add(id);
  }
  if (anyCustom) {
    for (const role of roles) {
      if (hasCustomRoleDefaults(role)) continue;
      if (role === "admin") return true;
      if (module.allowedRoles.some((r) => hasRole([role], r))) return true;
    }
    return fromCustom.has(module.id);
  }

  return module.allowedRoles.some((r) => hasRole(roles, r));
}

/** Modules enfants des hubs piliers (alignés sur DASHBOARD_PILLARS.moduleIds). */
const PILLAR_HUB_CHILD_MODULES: Record<string, string[]> = {
  "pillar-administratif": [
    "eleve-dossier",
    "notes",
    "groupes-pedagogiques",
    "stages",
    "agent-ia-ocr",
    "certificates",
  ],
  "pillar-etablissement": [
    "admin-settings",
    "organigramme",
    "evenements",
    "communication",
    "conformite-rgpd",
    "chatbot-knowledge",
  ],
  "pillar-services": [
    "travels",
    "prof-room",
    "requests-staff",
    "domain-planning",
    "documents",
    "toolbox",
    "channels",
    "assistance",
    "photocopies-couleur",
  ],
  "pillar-vie-scolaire": [
    "internat",
    "vs-appels",
    "vs-sanctions",
    "vs-carnet",
    "vs-calendrier",
    "groupes-pedagogiques",
  ],
  "pillar-compta-rh": ["rh", "mon-planning", "conformite-rgpd", "absences", "demandes-hse"],
};

export function canAccessIntranetPath(
  pathname: string,
  roles: string[],
  isOrgAdmin: boolean,
  access?: {
    byRole?: Record<string, { modules?: string[] }>;
    byUser?: Record<string, { modules?: string[] }>;
  } | null,
  userRef?: string | { userId?: string | null; businessUserId?: string | null } | null,
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

  return modules.some((m) => rolesAllowModule(roles, m, isOrgAdmin, access, userRef));
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
