/** Logique pure du job « nouveautés de l'ENT » : comparaison des compteurs (testable sans DB). */
import {
  findIncreasedSignals,
  toSignalCounts,
  type NotifiableSignal,
  type SignalCounts,
} from "@/app/components/notifications/notification-utils";

export type SignalDiff<T extends NotifiableSignal> = {
  /** Signaux à notifier (compteur en hausse). Vide au premier calcul (référence silencieuse). */
  toNotify: T[];
  /** Nouveaux compteurs à enregistrer (signaux disparus => absents, donc à supprimer). */
  nextCounts: SignalCounts;
  /** Identifiants dont la ligne doit être supprimée (disparus ou tombés à 0). */
  toDelete: string[];
};

/** `stored` = null : aucune ligne connue pour cet utilisateur (premier passage). */
export function diffSignals<T extends NotifiableSignal>(
  stored: SignalCounts | null,
  current: T[],
): SignalDiff<T> {
  const nextCounts: SignalCounts = {};
  for (const [id, count] of Object.entries(toSignalCounts(current))) {
    if (count > 0) nextCounts[id] = count;
  }
  const toDelete = stored ? Object.keys(stored).filter((id) => !(id in nextCounts)) : [];
  return { toNotify: findIncreasedSignals(stored, current), nextCounts, toDelete };
}

/** Lien relatif uniquement (jamais d'URL absolue ni protocole-relatif). */
export function safeRelativeHref(href: string): string {
  return href.startsWith("/") && !href.startsWith("//") ? href : "/";
}
