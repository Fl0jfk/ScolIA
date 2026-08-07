import type { ToolboxToolId } from "@/app/lib/toolbox-types";

export type ToolboxToolMeta = {
  id: ToolboxToolId;
  label: string;
  shortLabel: string;
  description: string;
  /** Route intranet (outil ou config) */
  adminPath: string;
  /** Page publique parents, si applicable */
  publicPath?: string;
  /** Couleur d'accent pour l'icône */
  color: string;
  bg: string;
  season?: string;
};

export const TOOLBOX_TOOLS_META: ToolboxToolMeta[] = [
  {
    id: "qrcreator",
    label: "QR Code",
    shortLabel: "QR",
    description: "Créer un QR code personnalisé avec le logo de l'établissement.",
    adminPath: "/qrcreator",
    color: "text-slate-800",
    bg: "bg-slate-100",
  },
  {
    id: "secret-santa",
    label: "Secret Santa",
    shortLabel: "Santa",
    description: "Tirage au sort anonyme pour l'équipe ou une classe.",
    adminPath: "/toolbox/secret-santa",
    color: "text-red-700",
    bg: "bg-red-50",
    season: "Décembre",
  },
  {
    id: "rentree",
    label: "Rentrée digitale",
    shortLabel: "Rentrée",
    description: "Hub familles : documents, simulateurs et liens utiles.",
    adminPath: "/toolbox?tab=rentree",
    publicPath: "/rentree",
    color: "text-amber-800",
    bg: "bg-amber-50",
    season: "Août–sept.",
  },
  {
    id: "simulateur-tarifs",
    label: "Simulateur tarifs",
    shortLabel: "Tarifs",
    description: "Barème scolarité configurable, page publique pour les parents.",
    adminPath: "/toolbox",
    publicPath: "/simulateurTarifs",
    color: "text-blue-800",
    bg: "bg-blue-50",
    season: "Rentrée",
  },
  {
    id: "simulateur-fournitures",
    label: "Fournitures",
    shortLabel: "Fourn.",
    description: "Liste fournitures par classe avec envoi par e-mail.",
    adminPath: "/toolbox?tab=fournitures",
    publicPath: "/simulateurFournitures",
    color: "text-emerald-800",
    bg: "bg-emerald-50",
    season: "Rentrée",
  },
  {
    id: "portes-ouvertes",
    label: "Portes ouvertes",
    shortLabel: "P. ouv.",
    description: "Inscriptions en ligne, créneaux, confirmation et calendrier.",
    adminPath: "/toolbox",
    publicPath: "/portes-ouvertes",
    color: "text-violet-800",
    bg: "bg-violet-50",
    season: "Sept.–janv.",
  },
  {
    id: "repartition-classes",
    label: "Répartition des classes",
    shortLabel: "Classes",
    description: "Préparer la classe, vœux parents et moteur de répartition.",
    adminPath: "/toolbox/repartition-classes",
    publicPath: "/repartition-classes",
    color: "text-indigo-800",
    bg: "bg-indigo-50",
    season: "Fin d'année",
  },
];

export function toolboxMetaById(id: ToolboxToolId): ToolboxToolMeta {
  return TOOLBOX_TOOLS_META.find((t) => t.id === id)!;
}

/** Liens admin permanents (pas de toggle on/off dans la config). */
export type ToolboxAdminLinkId = "parametres";

export type ToolboxAdminLinkMeta = {
  id: ToolboxAdminLinkId;
  label: string;
  shortLabel: string;
  description: string;
  adminPath: string;
  color: string;
  bg: string;
};

export const TOOLBOX_ADMIN_LINKS: ToolboxAdminLinkMeta[] = [
  {
    id: "parametres",
    label: "Paramètres généraux",
    shortLabel: "Param.",
    description: "Établissements, utilisateurs, identité, intégrations et référentiel scolaire.",
    adminPath: "/parametres",
    color: "text-slate-800",
    bg: "bg-slate-100",
  },
];
