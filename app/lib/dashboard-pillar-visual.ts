import type { DashboardPillarId } from "@/app/lib/dashboard-pillars";

/**
 * Orbe + dégradé latéral derrière chaque carte / hub de pilier.
 * Palette volontairement contrastée pour la reconnaissance rapide.
 */
export const PILLAR_ORB: Record<DashboardPillarId, string> = {
  administratif: "bg-red-500/45",
  etablissement: "bg-orange-400/45",
  services: "bg-amber-400/45",
  vie_scolaire: "bg-indigo-500/45",
  compta_rh: "bg-emerald-500/45",
  sante: "bg-fuchsia-400/45",
};

/** Dégradé horizontal qui traverse la tuile (gauche → transparent). */
export const PILLAR_WASH: Record<DashboardPillarId, string> = {
  administratif: "from-red-500/40 via-red-400/18",
  etablissement: "from-orange-400/40 via-orange-300/18",
  services: "from-amber-400/40 via-amber-300/18",
  vie_scolaire: "from-indigo-500/40 via-indigo-400/18",
  compta_rh: "from-emerald-500/40 via-emerald-400/18",
  sante: "from-fuchsia-400/40 via-fuchsia-300/18",
};

/** Filet coloré à gauche de la carte. */
export const PILLAR_EDGE: Record<DashboardPillarId, string> = {
  administratif: "bg-red-500",
  etablissement: "bg-orange-500",
  services: "bg-amber-500",
  vie_scolaire: "bg-indigo-500",
  compta_rh: "bg-emerald-500",
  sante: "bg-fuchsia-500",
};
