import type { DashboardPillarId } from "@/app/lib/dashboard-pillars";

/** Route native du module (hors hub `?tab=`). */
const PILLAR_MODULE_HREF: Record<string, string> = {
  "eleve-dossier": "/eleves/dossiers",
  notes: "/notes/espace",
  "notes-saisie": "/notes/saisie",
  "notes-bulletins": "/notes/bulletins",
  "notes-competences": "/notes/competences",
  "groupes-pedagogiques": "/groupes-pedagogiques",
  "facturation-familles": "/facturation",
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
  "edt-classe": "/edt-classe",
  "vs-calendrier": "/vie-scolaire/calendrier",
  "vs-appels": "/vie-scolaire/appels",
  "vs-absences": "/vie-scolaire/absences",
  "vs-sanctions": "/vie-scolaire/sanctions",
  "vs-carnet": "/vie-scolaire/carnet",
  "edt-etablissement": "/edt-etablissement",
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
    notes: "notes",
    saisie: "notes-saisie",
    bulletins: "notes-bulletins",
    competences: "notes-competences",
    groupes: "groupes-pedagogiques",
    stages: "stages",
    ocr: "agent-ia-ocr",
    certificates: "certificates",
    pilotage: "pilotage-eleves",
  },
  etablissement: {
    organigramme: "organigramme",
    evenements: "evenements",
    communication: "communication",
    parametres: "admin-settings",
    identite: "admin-settings",
    rgpd: "conformite-rgpd",
    brain: "chatbot-knowledge",
  },
  services: {
    travels: "travels",
    salles: "prof-room",
    demandes: "requests-staff",
    transversal: "domain-planning",
    cloud: "documents",
    toolbox: "toolbox",
    photocopies: "toolbox",
    salons: "channels",
    assistance: "assistance",
    covoiturage: "covoiturage",
  },
  vie_scolaire: {
    internat: "internat",
    calendrier: "vs-calendrier",
    appels: "vs-appels",
    absences: "vs-absences",
    sanctions: "vs-sanctions",
    carnet: "vs-carnet",
    groupes: "groupes-pedagogiques",
  },
  compta_rh: {
    rh: "rh",
    planning: "mon-planning",
    facturation: "facturation-familles",
  },
  sante: {
    sante: "sante",
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
  "notes-saisie": "✏️",
  "notes-bulletins": "📄",
  "notes-competences": "🎯",
  "facturation-familles": "💶",
  sante: "🩺",
  "bien-etre-referent": "💚",
  travels: "🚌",
  internat: "🌙",
  "vs-calendrier": "📆",
  "vs-appels": "✅",
  "vs-absences": "⏱️",
  "vs-sanctions": "⚠️",
  "vs-carnet": "📒",
  "pilotage-eleves": "📒",
  stages: "🎓",
  "agent-ia-ocr": "🤖",
  certificates: "📜",
  organigramme: "🗂️",
  evenements: "📅",
  communication: "📣",
  "mon-planning": "🗓️",
  "edt-etablissement": "🏫",
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
