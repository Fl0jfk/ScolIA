/**
 * Ordre par défaut des modules (référence pour la disposition initiale en 3 colonnes).
 * La feuille de semaine est hors grille (pied de page fixe).
 */
import {
  DESKTOP_BENTO_COLUMN_COUNT,
  linearOrderToColumns,
} from "@/app/lib/dashboard-bento-columns";

const DEFAULT_SORT = 50;

export const BENTO_DEFAULT_ORDER: string[] = [
  "documents",
  "travels",
  "agent-ia-ocr",
  "prof-room",
  "rh",
  "channels",
  "internat",
  "requests-staff",
  "organigramme",
  "toolbox",
  "domain-planning",
  "covoiturage",
  "assistance",
];

export const BENTO_MODULE_SORT: Record<string, number> = {
  documents: 1,
  travels: 2,
  "agent-ia-ocr": 3,
  "prof-room": 4,
  rh: 5,
  channels: 6,
  internat: 7,
  "requests-staff": 8,
  organigramme: 9,
  toolbox: 10,
  "domain-planning": 11,
  covoiturage: 12,
  "photocopies-couleur": 13,
  "chatbot-knowledge": 14,
  assistance: 15,
};

export function getBentoModuleSort(moduleId: string): number {
  return BENTO_MODULE_SORT[moduleId] ?? DEFAULT_SORT;
}

export function sortModuleIds(moduleIds: string[]): string[] {
  return [...moduleIds].sort((a, b) => {
    const sa = getBentoModuleSort(a);
    const sb = getBentoModuleSort(b);
    return sa - sb || a.localeCompare(b);
  });
}

export function defaultBentoModuleOrder(moduleIds: string[]): string[] {
  const set = new Set(moduleIds);
  const main = BENTO_DEFAULT_ORDER.filter((id) => set.has(id));
  const used = new Set(main);
  const rest = sortModuleIds(moduleIds.filter((id) => !used.has(id)));
  return [...main, ...rest];
}

export function defaultBentoModuleColumns(moduleIds: string[]): string[][] {
  return linearOrderToColumns(
    defaultBentoModuleOrder(moduleIds),
    DESKTOP_BENTO_COLUMN_COUNT,
  );
}
