import type { ToolboxToolId } from "@/app/lib/toolbox-types";

/** Outils pilotés uniquement depuis Établissement → Événements (plus dans la boîte à outils). */
export const EVENEMENTS_TOOL_IDS = ["portes-ouvertes", "rentree", "secret-santa"] as const;

export type EvenementToolId = (typeof EVENEMENTS_TOOL_IDS)[number];

function isEvenementToolId(id: ToolboxToolId | string): id is EvenementToolId {
  return (EVENEMENTS_TOOL_IDS as readonly string[]).includes(id);
}

/** Carte fixe (hors toolbox) — RDV inscription direction (Google Agenda). */
export const EVENEMENTS_RDV_INSCRIPTION_CARD = {
  title: "RDV inscription direction",
  description:
    "Prise de rendez-vous automatique avec la direction (école, collège, lycée) via Google Agenda — lien public pour les familles.",
  season: "Toute l’année",
  adminHref: "/etablissement/rdv-inscription",
  accent: "border-sky-200 bg-sky-50/80 text-sky-900",
} as const;

export const EVENEMENTS_TOOLS_META: {
  id: EvenementToolId;
  title: string;
  description: string;
  season: string;
  publicHref?: string;
  accent: string;
}[] = [
  {
    id: "portes-ouvertes",
    title: "Portes ouvertes",
    description: "Inscriptions en ligne, créneaux, confirmations et calendrier familles.",
    season: "Oct.–mars",
    publicHref: "/portes-ouvertes",
    accent: "border-violet-200 bg-violet-50/80 text-violet-900",
  },
  {
    id: "rentree",
    title: "Rentrée digitale",
    description: "Hub familles : documents, simulateurs et liens utiles pour la rentrée.",
    season: "Juin–sept.",
    publicHref: "/rentree",
    accent: "border-amber-200 bg-amber-50/80 text-amber-900",
  },
  {
    id: "secret-santa",
    title: "Secret Santa",
    description: "Tirage au sort anonyme pour l’équipe ou une classe.",
    season: "Décembre",
    accent: "border-rose-200 bg-rose-50/80 text-rose-900",
  },
];
