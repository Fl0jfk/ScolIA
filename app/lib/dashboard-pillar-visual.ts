import type { DashboardPillarId } from "@/app/lib/dashboard-pillars";

/**
 * Orbe de couleur derrière chaque carte / hub de pilier.
 * Palette volontairement contrastée pour la reconnaissance rapide.
 */
export const PILLAR_ORB: Record<DashboardPillarId, string> = {
  administratif: "bg-red-500/40",
  etablissement: "bg-orange-400/40",
  services: "bg-amber-400/40",
  vie_scolaire: "bg-indigo-500/40",
  compta_rh: "bg-emerald-500/40",
  sante: "bg-fuchsia-400/40",
};
