import type { ExternalQuickLinkConfig } from "@/app/lib/app-config-schemas";
import type { ExternalQuickLink } from "@/app/lib/intranet-modules";

export const DEFAULT_QUICK_LINK_ROLES = [
  "admin",
  "administratif",
  "professeur",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "comptabilite",
  "education",
  "cpe",
  "maintenance",
  "infirmerie",
] as const;

export type DashboardQuickLink = {
  id: string;
  name: string;
  link: string;
  img: string;
};

export function toDashboardQuickLinks(links: ExternalQuickLink[]): DashboardQuickLink[] {
  return links.map((l) => ({
    id: l.id,
    name: l.name,
    link: l.link,
    img: l.img || "",
  }));
}

export function newQuickLinkSlot(index: number): ExternalQuickLinkConfig {
  return {
    id: `link-${Date.now()}-${index}`,
    name: "",
    link: "",
    img: "",
    allowedRoles: [...DEFAULT_QUICK_LINK_ROLES],
  };
}
