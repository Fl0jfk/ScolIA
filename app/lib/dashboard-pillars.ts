import type { DashboardCategory } from "@/app/lib/intranet-modules";

export type DashboardPillarId = "eleves" | "rh" | "etablissement" | "services";

export type DashboardPillarDef = {
  id: DashboardPillarId;
  title: string;
  href: string;
  description: string;
  /** Modules intranet affichés sur la page hub / comme raccourcis de base. */
  moduleIds: string[];
};

export const DASHBOARD_PILLARS: DashboardPillarDef[] = [
  {
    id: "eleves",
    title: "Élèves",
    href: "/eleves",
    description: "",
    moduleIds: ["travels", "internat", "stages", "agent-ia-ocr", "certificates"],
  },
  {
    id: "rh",
    title: "RH",
    href: "/rh",
    description: "",
    moduleIds: ["rh"],
  },
  {
    id: "etablissement",
    title: "Établissement",
    href: "/etablissement",
    description: "",
    moduleIds: ["organigramme", "conformite-rgpd", "chatbot-knowledge", "domain-planning"],
  },
  {
    id: "services",
    title: "Services",
    href: "/services",
    description: "",
    moduleIds: [
      "requests-staff",
      "prof-room",
      "photocopies-couleur",
      "documents",
      "toolbox",
      "covoiturage",
      "channels",
      "assistance",
    ],
  },
];

export const DASHBOARD_FOOTER_ADMIN_MODULE_IDS = ["admin-settings", "admin-members"] as const;

export function categoriesForPillar(
  pillar: DashboardPillarDef,
  categories: DashboardCategory[],
): DashboardCategory[] {
  const order = new Map(pillar.moduleIds.map((id, i) => [id, i]));
  return categories
    .filter((c) => order.has(c.moduleId))
    .sort((a, b) => (order.get(a.moduleId) ?? 0) - (order.get(b.moduleId) ?? 0));
}

export function pillarHasVisibleModules(
  pillar: DashboardPillarDef,
  categories: DashboardCategory[],
): boolean {
  if (pillar.id === "rh") {
    return categories.some((c) => c.moduleId === "rh");
  }
  return categoriesForPillar(pillar, categories).length > 0;
}
