export const MESSAGING_MAX_DOCKED = 3;
export const MESSAGING_MAX_DOCKED_NARROW = 2;
export const MESSAGING_RECENT_LIST = 10;
export const MESSAGING_NARROW_MQ = "(max-width: 640px)";

/** Marqueur DOM pour ignorer les clics « extérieur » (fermeture overlay). */
export const MESSAGING_ROOT_ATTR = "data-scolia-messaging";
export const MESSAGING_ROOT_SELECTOR = `[${MESSAGING_ROOT_ATTR}]`;

/** Même taille / alignement que la bulle IA (h-14 w-14, right-4). */
export const MESSAGING_FAB_CLASS =
  "flex h-14 w-14 items-center justify-center overflow-hidden rounded-full shadow-[0_14px_34px_rgba(15,23,42,0.35)] transition-all hover:scale-[1.04] active:scale-[0.97]";

/** Chat heads : pas d’overflow pour laisser passer anneau + pastille de présence. */
export const MESSAGING_HEAD_CLASS =
  "flex h-14 w-14 items-center justify-center rounded-full shadow-[0_14px_34px_rgba(15,23,42,0.35)] transition-all hover:scale-[1.04] active:scale-[0.97]";

/** Au-dessus de l’IA (bottom-4 + h-14 + gap). */
export const MESSAGING_FAB_POSITION = "fixed bottom-[5.25rem] right-4 z-[129]";

const STORAGE_PREFIX = "scolia.messaging.dock.";
const HEADS_EXPANDED_PREFIX = "scolia.messaging.headsExpanded.";

export function messagingDockStorageKey(userId: string, etablissementId?: string): string {
  return `${STORAGE_PREFIX}${etablissementId ?? "default"}.${userId}`;
}

export function messagingHeadsExpandedKey(userId: string, etablissementId?: string): string {
  return `${HEADS_EXPANDED_PREFIX}${etablissementId ?? "default"}.${userId}`;
}

export function readDockedConversationIds(storageKey: string): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((x): x is string => typeof x === "string").slice(0, MESSAGING_MAX_DOCKED);
  } catch {
    return [];
  }
}

export function writeDockedConversationIds(storageKey: string, ids: string[]): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey, JSON.stringify(ids.slice(0, MESSAGING_MAX_DOCKED)));
  } catch {
    /* ignore quota */
  }
}

export function readHeadsExpanded(storageKey: string): boolean {
  if (typeof window === "undefined") return false;
  try {
    return window.localStorage.getItem(storageKey) === "1";
  } catch {
    return false;
  }
}

export function writeHeadsExpanded(storageKey: string, expanded: boolean): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey, expanded ? "1" : "0");
  } catch {
    /* ignore */
  }
}

/** Ajoute une conversation en tête du dock (MRU), plafonné à max. */
export function pushDockedConversation(
  current: string[],
  conversationId: string,
  max: number = MESSAGING_MAX_DOCKED,
): string[] {
  const next = [conversationId, ...current.filter((id) => id !== conversationId)];
  return next.slice(0, max);
}

export function removeDockedConversation(current: string[], conversationId: string): string[] {
  return current.filter((id) => id !== conversationId);
}
