import type { DashboardPillarId } from "@/app/lib/dashboard-pillars";

/** Route native du module (hors hub `?tab=`). */
const PILLAR_MODULE_HREF: Record<string, string> = {
  "eleve-dossier": "/eleves/dossiers",
  notes: "/notes/espace",
  sante: "/sante/espace",
  "bien-etre-referent": "/bien-etre/referent",
  travels: "/travels",
  internat: "/gestion-internat",
  "pilotage-eleves": "/pilotage-eleves",
  stages: "/stages",
  "agent-ia-ocr": "/agentIAOCR",
  certificates: "/certificates",
  organigramme: "/organigramme",
  evenements: "/etablissement/evenements",
  communication: "/etablissement/communication",
  identite: "/parametres?tab=site",
  "mon-planning": "/mon-planning",
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
  rh: "/rh",
};

/** Anciens onglets hub → moduleId (compat `?tab=`). */
const PILLAR_TAB_TO_MODULE: Record<DashboardPillarId, Record<string, string>> = {
  administratif: {
    dossiers: "eleve-dossier",
    travels: "travels",
    stages: "stages",
    ocr: "agent-ia-ocr",
    certificates: "certificates",
    pilotage: "pilotage-eleves",
    organigramme: "organigramme",
    evenements: "evenements",
    communication: "communication",
    parametres: "admin-settings",
    rgpd: "conformite-rgpd",
    brain: "chatbot-knowledge",
    demandes: "requests-staff",
    cloud: "documents",
    toolbox: "toolbox",
    salons: "channels",
    assistance: "assistance",
  },
  vie_scolaire: {
    internat: "internat",
    travels: "travels",
    stages: "stages",
    dossiers: "eleve-dossier",
    salles: "prof-room",
    transversal: "domain-planning",
    demandes: "requests-staff",
  },
  notes: {
    notes: "notes",
    pilotage: "pilotage-eleves",
    dossiers: "eleve-dossier",
    salles: "prof-room",
  },
  compta_rh: {
    rh: "rh",
    planning: "mon-planning",
    cloud: "documents",
    travels: "travels",
  },
  sante: {
    sante: "sante",
    dossiers: "eleve-dossier",
    "bien-etre": "bien-etre-referent",
  },
};

/** Legacy hubs → mapping tab pour redirects. */
const LEGACY_HUB_TAB_TO_MODULE: Record<string, Record<string, string>> = {
  eleves: {
    dossiers: "eleve-dossier",
    travels: "travels",
    internat: "internat",
    pilotage: "pilotage-eleves",
    stages: "stages",
    ocr: "agent-ia-ocr",
    certificates: "certificates",
  },
  etablissement: {
    organigramme: "organigramme",
    evenements: "evenements",
    communication: "communication",
    identite: "admin-settings",
    parametres: "admin-settings",
    rgpd: "conformite-rgpd",
    brain: "chatbot-knowledge",
    transversal: "domain-planning",
  },
  services: {
    ocr: "agent-ia-ocr",
    demandes: "requests-staff",
    salles: "prof-room",
    photocopies: "toolbox",
    transversal: "domain-planning",
    cloud: "documents",
    toolbox: "toolbox",
    covoiturage: "covoiturage",
    salons: "channels",
    assistance: "assistance",
  },
  rh: {
    planning: "mon-planning",
  },
};

export function moduleHref(moduleId: string, fallback = "/dashboard"): string {
  return PILLAR_MODULE_HREF[moduleId] || fallback;
}

/** Résout une URL `?tab=` historique vers la route module, ou null. */
export function resolveLegacyPillarTab(
  pillarId: string,
  tab: string | null | undefined,
): string | null {
  if (!tab) return null;
  const map =
    PILLAR_TAB_TO_MODULE[pillarId as DashboardPillarId] ||
    LEGACY_HUB_TAB_TO_MODULE[pillarId];
  if (!map) return null;
  const moduleId = map[tab];
  if (!moduleId) return null;
  return moduleHref(moduleId);
}

export const MODULE_EMOJI: Record<string, string> = {
  "eleve-dossier": "📁",
  notes: "📝",
  sante: "🩺",
  "bien-etre-referent": "💚",
  travels: "🚌",
  internat: "🌙",
  "pilotage-eleves": "📒",
  stages: "🎓",
  "agent-ia-ocr": "🤖",
  certificates: "📜",
  organigramme: "🗂️",
  evenements: "📅",
  communication: "📣",
  "mon-planning": "🗓️",
  "conformite-rgpd": "🔒",
  "chatbot-knowledge": "🧠",
  "domain-planning": "🧭",
  "requests-staff": "📥",
  "prof-room": "🚪",
  documents: "☁️",
  toolbox: "🧰",
  covoiturage: "🚗",
  channels: "💬",
  assistance: "🆘",
  "admin-settings": "⚙️",
  rh: "👥",
};
