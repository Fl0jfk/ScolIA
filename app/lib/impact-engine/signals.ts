import "server-only";

import type { CreneauVideSignal } from "./types";

/**
 * File interne direction / CPE — signaux créneaux vidés.
 * Calcul à la volée + cache process (pas de table `vs_signal` en lot 1).
 * ENABLE_VS_* reste off : pas d’UI appels.
 */

const signalsByEtab = new Map<string, CreneauVideSignal[]>();

export function listCreneauVideSignals(etablissementId: string): CreneauVideSignal[] {
  return [...(signalsByEtab.get(etablissementId) ?? [])];
}

export function listCreneauVideSignalsForTravel(
  etablissementId: string,
  travelId: string,
): CreneauVideSignal[] {
  return listCreneauVideSignals(etablissementId).filter((s) => s.travelId === travelId);
}

/** Remplace les signaux d’un voyage (confirm / refresh). N’écrit pas le planning. */
export function replaceCreneauVideSignalsForTravel(
  etablissementId: string,
  travelId: string,
  next: CreneauVideSignal[],
): void {
  const others = listCreneauVideSignals(etablissementId).filter((s) => s.travelId !== travelId);
  signalsByEtab.set(etablissementId, [...others, ...next]);
}

export function clearCreneauVideSignalsForTravel(
  etablissementId: string,
  travelId: string,
): void {
  replaceCreneauVideSignalsForTravel(etablissementId, travelId, []);
}

/** Tests / reset process. */
export function resetCreneauVideSignalsStore(): void {
  signalsByEtab.clear();
}
