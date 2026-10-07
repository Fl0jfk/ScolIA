/** Chemins supervision toujours autorisés (hors matrice modules cible). */
export function isSupervisionApiPath(pathname: string): boolean {
  return pathname === "/api/supervision" || pathname.startsWith("/api/supervision/");
}

/** Mutations autorisées en mode supervision (sortie uniquement). */
export function isSupervisionWriteAllowedPath(pathname: string): boolean {
  return pathname === "/api/supervision/stop";
}

/**
 * Messagerie / salons privés : interdits en supervision.
 * La messagerie reste liée au compte acteur (pas à la cible) — risque de confusion
 * et de lecture de conversations personnelles hors périmètre.
 * Exception : `/api/channels/users/list` (annuaire utilisé par d’autres modules RH).
 */
export function isSupervisionPrivacyBlockedPath(pathname: string): boolean {
  if (
    pathname === "/api/channels/users/list" ||
    pathname.startsWith("/api/channels/users/list?")
  ) {
    return false;
  }
  return (
    pathname === "/messagerie" ||
    pathname.startsWith("/messagerie/") ||
    pathname === "/api/messaging" ||
    pathname.startsWith("/api/messaging/") ||
    pathname === "/channels" ||
    pathname.startsWith("/channels/") ||
    pathname === "/api/channels" ||
    pathname.startsWith("/api/channels/")
  );
}
