import { getParisParts } from "@/app/lib/paris-time";
import { isDateSortiePassee } from "@/app/lib/siecle-eleves-parse";

function normalizeStatus(raw: unknown): string {
  return String(raw ?? "").trim().toLowerCase();
}

/** Champs minimaux pour décider si l’élève apparaît dans les listes / effectifs / modules. */
export type EleveActifListesFields = {
  status?: string | null;
  /** AAAA-MM-JJ (colonne `eleve.date_sortie` ou import Siècle / Excel). */
  dateSortie?: string | null;
};

/**
 * Élève actif pour toutes les lectures « liste » (classes, stages, appel, cantine, recherche…).
 * Hors fiche dossier ouverte par id (là : `isEleveSortantEtablissement` pour l’affichage).
 */
export function isEleveActifPourListes(
  fields: EleveActifListesFields,
  now: Date = new Date(),
): boolean {
  const s = normalizeStatus(fields.status);
  if (s && s !== "inscrit") return false;
  if (isDateSortiePassee(fields.dateSortie, now)) return false;
  return true;
}

/** Dossier individuel : élève sorti (statut ou date de sortie ≤ aujourd’hui). */
export function isEleveSortantEtablissement(
  fields: EleveActifListesFields,
  now: Date = new Date(),
): boolean {
  const s = normalizeStatus(fields.status);
  if (s === "ancien" || s === "archive") return true;
  return isDateSortiePassee(fields.dateSortie, now);
}

/** Secondes jusqu’à minuit Paris (pour TTL cache registre actifs). */
export function secondsUntilParisMidnight(now: Date = new Date()): number {
  const p = getParisParts(now);
  const elapsed = p.hour * 3600 + p.minute * 60 + p.second;
  const remaining = 24 * 3600 - elapsed;
  return Math.max(60, remaining);
}

export function ttlElevesActifsRegistryCache(capSeconds: number, now: Date = new Date()): number {
  return Math.min(capSeconds, secondsUntilParisMidnight(now));
}

export function formatDateSortieFromRow(raw: string | Date | null | undefined): string | null {
  if (raw == null) return null;
  if (raw instanceof Date) {
    const y = raw.getFullYear();
    const m = String(raw.getMonth() + 1).padStart(2, "0");
    const d = String(raw.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }
  const s = String(raw).trim();
  return s || null;
}
