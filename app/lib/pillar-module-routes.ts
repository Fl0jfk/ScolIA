import type { DashboardPillarId } from "@/app/lib/dashboard-pillars";

/** Route native du module (hors hub `?tab=`). */
export const PILLAR_MODULE_HREF: Record<string, string> = {
  travels: "/travels",
  internat: "/gestion-internat",
  stages: "/stages",
  "agent-ia-ocr": "/agentIAOCR",
  certificates: "/certificates",
  organigramme: "/organigramme",
  "conformite-rgpd": "/conformite-rgpd",
  "chatbot-knowledge": "/chatbot-knowledge",
  "domain-planning": "/domain-planning",
  "requests-staff": "/requests",
  "prof-room": "/prof-room",
  "photocopies-couleur": "/photocopies-couleur",
  documents: "/documents",
  toolbox: "/toolbox",
  covoiturage: "/covoiturage",
  channels: "/channels",
  assistance: "/assistance",
  "admin-settings": "/parametres",
};

/** Anciens onglets hub → moduleId (compat `?tab=`). */
export const PILLAR_TAB_TO_MODULE: Record<
  DashboardPillarId,
  Record<string, string>
> = {
  eleves: {
    travels: "travels",
    internat: "internat",
    stages: "stages",
    ocr: "agent-ia-ocr",
    certificates: "certificates",
  },
  etablissement: {
    organigramme: "organigramme",
    rgpd: "conformite-rgpd",
    brain: "chatbot-knowledge",
    transversal: "domain-planning",
  },
  services: {
    demandes: "requests-staff",
    salles: "prof-room",
    photocopies: "photocopies-couleur",
    cloud: "documents",
    toolbox: "toolbox",
    covoiturage: "covoiturage",
    salons: "channels",
    assistance: "assistance",
  },
  rh: {},
};

export function moduleHref(moduleId: string, fallback = "/dashboard"): string {
  return PILLAR_MODULE_HREF[moduleId] || fallback;
}

/** Résout une URL `?tab=` historique vers la route module, ou null. */
export function resolveLegacyPillarTab(
  pillarId: DashboardPillarId,
  tab: string | null | undefined,
): string | null {
  if (!tab) return null;
  const moduleId = PILLAR_TAB_TO_MODULE[pillarId]?.[tab];
  if (!moduleId) return null;
  return moduleHref(moduleId);
}

export const MODULE_EMOJI: Record<string, string> = {
  travels: "🚌",
  internat: "🌙",
  stages: "📝",
  "agent-ia-ocr": "📄",
  certificates: "🏅",
  organigramme: "🗺️",
  "conformite-rgpd": "🔒",
  "chatbot-knowledge": "🧠",
  "domain-planning": "📚",
  "requests-staff": "📨",
  "prof-room": "🚪",
  "photocopies-couleur": "🖨️",
  documents: "☁️",
  toolbox: "🧰",
  covoiturage: "🚗",
  channels: "💬",
  assistance: "🆘",
  rh: "👥",
  absences: "📅",
  "demandes-hse": "⏱️",
  "admin-settings": "⚙️",
  "week-sheet": "📰",
};
