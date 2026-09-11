/**
 * Préfixes de clés Valkey — toujours scopés établissement / user.
 * TTL en secondes (données métier : courts ; configs : moyens).
 */

export const VALKEY_TTL = {
  /** Gate MFA / présence passkey */
  passkeyPresence: 60,
  /** Snapshot proxy auth (rôles, flags) */
  proxyAuth: 45,
  /** Memberships user (canal staff / famille) */
  memberships: 60,
  /** Liste conversations messagerie */
  messagingConversations: 20,
  /** Page messages (très courte — fraîcheur chat) */
  messagingMessages: 8,
  /** Liste dossiers élèves (filtres inclus dans la clé) */
  elevesDossiersList: 30,
  /** Module-access effectif user */
  moduleAccessUser: 45,
  /** Liens dashboard */
  dashboardLinks: 60,
  /** Signaux dashboard (agrégat lourd) */
  dashboardSignals: 25,
  /** Config app / module-access store */
  appConfig: 45,
  moduleAccessConfig: 30,
  /** Rate-limit applicatif */
  rateLimit: 0, // TTL = fenêtre passée à incr
  /** Job import photos (état + progression) */
  photoJob: 6 * 60 * 60,
  /** Roster matching photos (1 lecture BDD / job) */
  photoJobRoster: 2 * 60 * 60,
} as const;

const NS = "scola";

export function valkeyKeyPasskey(userId: string): string {
  return `${NS}:passkey:${userId}`;
}

export function valkeyKeyProxyAuth(
  authUserId: string,
  etablissementId: string | null,
): string {
  return `${NS}:proxy:${authUserId}:${etablissementId ?? "_"}`;
}

export function valkeyKeyMemberships(userId: string): string {
  return `${NS}:memberships:${userId}`;
}

export function valkeyKeyMessagingConversations(
  etablissementId: string,
  userId: string,
): string {
  return `${NS}:msg:convs:${etablissementId}:${userId}`;
}

export function valkeyKeyMessagingMessages(
  etablissementId: string,
  conversationId: string,
  cursor: string,
): string {
  return `${NS}:msg:msgs:${etablissementId}:${conversationId}:${cursor || "head"}`;
}

export function valkeyKeyElevesDossiersList(parts: {
  etablissementId: string;
  viewerKey: string;
  siteId?: string;
  classe?: string;
  status?: string;
  metaOnly?: boolean;
}): string {
  return [
    NS,
    "eleves:list",
    parts.etablissementId,
    parts.viewerKey,
    parts.metaOnly ? "meta" : "full",
    parts.siteId || "-",
    parts.classe || "-",
    parts.status || "-",
  ].join(":");
}

/** Préfixe pour invalider toutes les listes dossiers d’un établissement. */
export function valkeyPrefixElevesDossiers(etablissementId: string): string {
  return `${NS}:eleves:list:${etablissementId}:`;
}

export function valkeyKeyModuleAccessUser(
  etablissementId: string,
  userId: string,
): string {
  return `${NS}:mods:${etablissementId}:${userId}`;
}

export function valkeyKeyDashboardLinks(etablissementId: string): string {
  return `${NS}:dash:links:${etablissementId}`;
}

export function valkeyKeyDashboardSignals(
  etablissementId: string,
  userId: string,
): string {
  return `${NS}:dash:sig:${etablissementId}:${userId}`;
}

export function valkeyKeyAppConfig(tenantSlug: string): string {
  return `${NS}:cfg:app:${tenantSlug || "default"}`;
}

export function valkeyKeyModuleAccessConfig(tenantSlug: string): string {
  return `${NS}:cfg:mods:${tenantSlug || "default"}`;
}

export function valkeyKeyRateLimit(key: string): string {
  return `${NS}:rl:${key}`;
}

/** État d’un job d’import photos élèves (gros JSON — hors hot path auth). */
export function valkeyKeyPhotoJob(jobId: string): string {
  return `${NS}:photos:job:${jobId}`;
}

/** Roster slim (id/nom/prénom/ine) pour matching photos — 1 charge BDD / job. */
export function valkeyKeyPhotoJobRoster(jobId: string): string {
  return `${NS}:photos:roster:${jobId}`;
}
