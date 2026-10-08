/**
 * Catalogue de destinations ScolIA — navigation naturelle sur tout l’intranet.
 * Dérivé du catalogue modules + deep links métier connus.
 */

import { INTRANET_MODULES } from "@/app/lib/intranet-modules";
import { moduleHref } from "@/app/lib/pillar-module-routes";

export type ScoliaDestination = {
  id: string;
  label: string;
  href: string;
  moduleId: string;
  keywords: string[];
  /** Si true, le client ouvre en overlay iframe / surface plutôt qu’en navigation. */
  preferModal?: boolean;
};

function uniqKeywords(...parts: Array<string | undefined | null>): string[] {
  const out = new Set<string>();
  for (const p of parts) {
    const raw = String(p || "")
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "");
    for (const token of raw.split(/[^a-z0-9]+/).filter((t) => t.length >= 2)) {
      out.add(token);
    }
    if (raw.trim()) out.add(raw.trim());
  }
  return [...out];
}

const EXTRA_DESTINATIONS: ScoliaDestination[] = [
  {
    id: "eleves-dossiers",
    label: "Dossiers élèves",
    href: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    keywords: uniqKeywords("dossiers eleves", "liste eleves", "inscriptions", "preinscription"),
  },
  {
    id: "accueil-absences",
    label: "Absences accueil",
    href: "/accueil/absences",
    moduleId: "accueil-absences",
    keywords: uniqKeywords("absences", "accueil", "vie scolaire absences"),
  },
  {
    id: "vie-scolaire-absences",
    label: "Absences vie scolaire",
    href: "/vie-scolaire/absences",
    moduleId: "accueil-absences",
    keywords: uniqKeywords("absences eleves", "vie scolaire"),
  },
  {
    id: "travels",
    label: "Sorties scolaires",
    href: "/travels",
    moduleId: "travels",
    keywords: uniqKeywords("sorties", "voyages", "sejours", "travels"),
  },
  {
    id: "prof-room",
    label: "Réservation de salles",
    href: "/prof-room",
    moduleId: "prof-room",
    keywords: uniqKeywords("salles", "reservation", "occuper une salle"),
  },
  {
    id: "requests",
    label: "Demandes staff",
    href: "/requests",
    moduleId: "requests-staff",
    keywords: uniqKeywords("demandes", "tickets", "requests"),
  },
  {
    id: "photocopies",
    label: "Photocopies",
    href: "/photocopies",
    moduleId: "photocopies-couleur",
    keywords: uniqKeywords("photocopies", "impression", "copies"),
  },
  {
    id: "stages",
    label: "Stages",
    href: "/stages",
    moduleId: "stages",
    keywords: uniqKeywords("stages", "conventions", "entreprises"),
  },
  {
    id: "internat",
    label: "Internat",
    href: "/gestion-internat",
    moduleId: "internat",
    keywords: uniqKeywords("internat", "chambres", "appel du soir"),
  },
  {
    id: "rh-absences",
    label: "Absences RH",
    href: "/rh?tab=dashboard&section=absences&view=a-traiter",
    moduleId: "rh",
    keywords: uniqKeywords(
      "absences profs",
      "rh",
      "personnel",
      "valider absences",
      "direction absences",
    ),
  },
  {
    id: "hse",
    label: "Demandes HSE",
    href: "/rh?tab=dashboard&section=hse",
    moduleId: "demandes-hse",
    keywords: uniqKeywords("hse", "securite", "hygiene"),
  },
  {
    id: "documents",
    label: "Cloud documents",
    href: "/documents",
    moduleId: "documents",
    keywords: uniqKeywords("cloud", "documents", "fichiers"),
  },
  {
    id: "scolia-ai",
    label: "ScolIA plein écran",
    href: "/scolia-ai",
    moduleId: "chatbot-knowledge",
    keywords: uniqKeywords("scolia", "assistant", "ia"),
  },
];

function moduleDestinations(): ScoliaDestination[] {
  const out: ScoliaDestination[] = [];
  for (const mod of INTRANET_MODULES) {
    if (!mod.dashboard?.link || mod.dashboard.external) continue;
    const href = moduleHref(mod.id, mod.dashboard.link) || mod.dashboard.link;
    if (!href.startsWith("/")) continue;
    out.push({
      id: `module-${mod.id}`,
      label: mod.dashboard.name,
      href,
      moduleId: mod.id,
      keywords: uniqKeywords(mod.id, mod.dashboard.name, mod.dashboard.description),
    });
  }
  return out;
}

let cached: ScoliaDestination[] | null = null;

export function listScoliaDestinations(): ScoliaDestination[] {
  if (cached) return cached;
  const byHref = new Map<string, ScoliaDestination>();
  for (const d of [...EXTRA_DESTINATIONS, ...moduleDestinations()]) {
    const key = d.href.split("?")[0] || d.href;
    if (!byHref.has(key)) byHref.set(key, d);
  }
  cached = [...byHref.values()];
  return cached;
}

function fold(s: string): string {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function searchScoliaDestinations(
  query: string,
  limit = 8,
): ScoliaDestination[] {
  const q = fold(query);
  if (!q) return listScoliaDestinations().slice(0, limit);
  const tokens = q.split(" ").filter(Boolean);
  const scored = listScoliaDestinations()
    .map((d) => {
      const hay = fold([d.label, d.moduleId, ...d.keywords].join(" "));
      let score = 0;
      if (hay.includes(q)) score += 10;
      for (const t of tokens) {
        if (hay.includes(t)) score += 3;
        if (d.keywords.some((k) => k.includes(t))) score += 2;
      }
      return { d, score };
    })
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score);
  return scored.slice(0, limit).map((x) => x.d);
}

export function findScoliaDestinationById(id: string): ScoliaDestination | undefined {
  return listScoliaDestinations().find((d) => d.id === id);
}
