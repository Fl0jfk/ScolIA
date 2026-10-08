import type { Establishment } from "@/app/lib/app-config-schemas";
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import {
  directionRoleForKind,
  isAnyDirectionRole,
  isGroupeScolaireRef,
  matchEstablishment,
  roleSlugsForEstablishment,
  userIsAnyDirection,
} from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { hasRole, normRole } from "@/app/lib/intranet-role-utils";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";

export type TravelThreadTripRef = {
  ownerId?: string | null;
  ownerName?: string | null;
  data?: { etablissement?: string | null };
};

export type TravelThreadViewer = {
  user: SessionLikeUser | null | undefined;
  roles: string[];
  extraUserIds?: string[];
};

/**
 * Libellé expéditeur du fil interne séjours : prénom + nom de la personne,
 * pas le rôle (« Comptabilité », « Direction », …).
 */
export function formatTravelMessageAuthorLabel(input: {
  firstName?: string | null;
  lastName?: string | null;
  name?: string | null;
  email?: string | null;
}): string {
  const fromParts = [input.firstName, input.lastName]
    .map((part) => String(part ?? "").trim())
    .filter(Boolean)
    .join(" ")
    .trim();
  if (fromParts) return fromParts;

  const name = String(input.name ?? "").trim();
  if (name && !/^utilisateur$/i.test(name)) return name;

  const email = String(input.email ?? "").trim();
  if (email.includes("@")) {
    const local = email.split("@")[0]?.trim();
    if (local) return local;
  } else if (email) {
    return email;
  }
  return "Utilisateur";
}

export function travelThreadViewerFromStaff(user: {
  id: string;
  businessUserId?: string | null;
  externalUserId?: string | null;
  name?: string | null;
  firstName?: string | null;
  lastName?: string | null;
  email?: string | null;
  roles: string[];
}): TravelThreadViewer {
  const fullName = formatTravelMessageAuthorLabel(user);
  return {
    user: {
      id: user.id,
      fullName: fullName === "Utilisateur" ? null : fullName,
      publicMetadata: { role: user.roles },
    },
    roles: user.roles,
    extraUserIds: [user.businessUserId, user.externalUserId].filter(
      (v): v is string => Boolean(v?.trim()),
    ),
  };
}

export type TravelThreadMessageRef = {
  authorUserId?: string | null;
  date?: string | null;
};

function viewerIdSet(viewer: TravelThreadViewer): Set<string> {
  const ids = new Set<string>();
  for (const raw of [viewer.user?.id, ...(viewer.extraUserIds ?? [])]) {
    const id = String(raw || "").trim();
    if (id) ids.add(id);
  }
  return ids;
}

export function viewerIsCompta(roles: string[]): boolean {
  return hasRole(roles, "comptabilite") || hasRole(roles, "compta");
}

export function isTripOwnerForThread(
  trip: TravelThreadTripRef,
  viewer: TravelThreadViewer,
): boolean {
  const ids = viewerIdSet(viewer);
  const ownerId = String(trip.ownerId || "").trim();
  if (ownerId && ids.has(ownerId)) return true;
  return isTripOwnerOrCreator(trip, viewer.user);
}

/** Peut écrire / lire le fil (créateur, toute direction, compta). */
export function canUseTravelInternalThread(
  trip: TravelThreadTripRef,
  viewer: TravelThreadViewer,
  establishments: Establishment[],
): boolean {
  if (isTripOwnerForThread(trip, viewer)) return true;
  if (viewerIsCompta(viewer.roles)) return true;
  if (userIsAnyDirection(viewer.user, establishments, viewer.roles, viewer.extraUserIds)) {
    return true;
  }
  if (isAnyDirectionRole(viewer.roles)) return true;
  return false;
}

/**
 * Direction « concernée » par le séjour : rôle exact du site (ou directrice
 * configurée). Pas de match flou — la direction lycée ne reçoit pas les
 * messages collège, et inversement. Groupe scolaire → toutes les directions.
 */
export function viewerIsDirectionForTravelTrip(
  trip: TravelThreadTripRef,
  viewer: TravelThreadViewer,
  establishments: Establishment[],
): boolean {
  const etab = trip.data?.etablissement;
  if (isGroupeScolaireRef(etab)) {
    return (
      isAnyDirectionRole(viewer.roles) ||
      userIsAnyDirection(viewer.user, establishments, viewer.roles, viewer.extraUserIds)
    );
  }

  const est = matchEstablishment(establishments, etab);
  if (!est) return false;

  const directorId = est.directorExternalUserId?.trim();
  if (directorId && viewerIdSet(viewer).has(directorId)) return true;

  const wanted = new Set<string>(
    (est.roleSlugs && est.roleSlugs.length > 0
      ? est.roleSlugs
      : roleSlugsForEstablishment(est)
    ).map((slug) => normRole(slug)),
  );
  wanted.add(normRole(directionRoleForKind(inferEstablishmentKind(est))));

  // Correspondance exacte uniquement (évite « direction » ⊃ tous les sites).
  return viewer.roles.some((role) => wanted.has(normRole(role)));
}

/**
 * Doit être notifié d’un message non lu : direction du site du séjour
 * (toutes les directions si Groupe scolaire), toute la compta, créateur.
 */
export function viewerIsTravelThreadAudience(
  trip: TravelThreadTripRef,
  viewer: TravelThreadViewer,
  establishments: Establishment[],
): boolean {
  if (isTripOwnerForThread(trip, viewer)) return true;
  if (viewerIsCompta(viewer.roles)) return true;
  return viewerIsDirectionForTravelTrip(trip, viewer, establishments);
}

export function messageIsUnreadForViewer(params: {
  message: TravelThreadMessageRef;
  lastReadAt: Date | null;
  viewerUserIds: Iterable<string>;
}): boolean {
  const author = String(params.message.authorUserId || "").trim();
  const ids = new Set(
    [...params.viewerUserIds].map((id) => id.trim()).filter(Boolean),
  );
  if (author && ids.has(author)) return false;
  const raw = params.message.date?.trim();
  if (!raw) return Boolean(params.lastReadAt == null);
  const at = new Date(raw);
  if (Number.isNaN(at.getTime())) return params.lastReadAt == null;
  if (!params.lastReadAt) return true;
  return at.getTime() > params.lastReadAt.getTime();
}

export function countUnreadTravelMessages(params: {
  messages: TravelThreadMessageRef[];
  lastReadAt: Date | null;
  viewerUserIds: Iterable<string>;
}): number {
  return params.messages.filter((message) =>
    messageIsUnreadForViewer({
      message,
      lastReadAt: params.lastReadAt,
      viewerUserIds: params.viewerUserIds,
    }),
  ).length;
}

export function summarizeTravelUnread(countsByTrip: Map<string, number>): {
  tripCount: number;
  messageCount: number;
  firstTripId: string | null;
} {
  let tripCount = 0;
  let messageCount = 0;
  let firstTripId: string | null = null;
  for (const [tripId, n] of countsByTrip) {
    if (n <= 0) continue;
    tripCount += 1;
    messageCount += n;
    if (!firstTripId) firstTripId = tripId;
  }
  return { tripCount, messageCount, firstTripId };
}
