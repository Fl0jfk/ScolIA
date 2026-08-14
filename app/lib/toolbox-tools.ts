import type { ToolboxToolId } from "@/app/lib/toolbox-types";

type ToolboxToolMeta = {
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

/** Outils restants dans la boîte à outils. */
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

/** Liens permanents du hub (pas de toggle on/off). */
export type ToolboxHubLinkId = "photocopies-couleur";

export type ToolboxHubLinkMeta = {
  id: ToolboxHubLinkId;
  label: string;
  shortLabel: string;
  description: string;
  adminPath: string;
  color: string;
  bg: string;
};

export const TOOLBOX_HUB_LINKS: ToolboxHubLinkMeta[] = [
  {
    id: "photocopies-couleur",
    label: "Photocopies couleur",
    shortLabel: "Photo.",
    description: "Demander une impression couleur — validation direction puis service impressions.",
    adminPath: "/photocopies-couleur",
    color: "text-fuchsia-800",
    bg: "bg-fuchsia-50",
  },
];

function toolboxMetaById(id: ToolboxToolId): ToolboxToolMeta | undefined {
  return TOOLBOX_TOOLS_META.find((t) => t.id === id);
}

/** Liens admin permanents (pas de toggle on/off dans la config). */
export type ToolboxAdminLinkId = "parametres" | "evenements" | "communication";

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
    label: "Paramètres",
    shortLabel: "Param.",
    description: "Utilisateurs, établissement, liste des élèves et réglages globaux.",
    adminPath: "/parametres",
    color: "text-slate-800",
    bg: "bg-slate-100",
  },
  {
    id: "evenements",
    label: "Événements",
    shortLabel: "Évén.",
    description: "Portes ouvertes, rentrée digitale et Secret Santa.",
    adminPath: "/etablissement/evenements",
    color: "text-amber-800",
    bg: "bg-amber-50",
  },
  {
    id: "communication",
    label: "Communication",
    shortLabel: "Com.",
    description: "Simulateur de tarifs et outils diffusables en ligne.",
    adminPath: "/etablissement/communication",
    color: "text-sky-800",
    bg: "bg-sky-50",
  },
];
