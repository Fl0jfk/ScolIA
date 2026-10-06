import type { Establishment } from "@/app/lib/app-config-schemas";
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import {
  isAnyDirectionRole,
  isGroupeScolaireRef,
  userCanActAsDirectionFor,
  userIsAnyDirection,
} from "@/app/lib/establishment-catalog";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";

export type TravelThreadTripRef = {
  ownerId?: string | null;
  ownerName?: string | null;
  data?: { etablissement?: string | null };
};

export function travelThreadViewerFromStaff(user: {
  id: string;
  businessUserId?: string | null;
  externalUserId?: string | null;
  name?: string | null;
  firstName?: string | null;
  lastName?: string | null;
  roles: string[];
}): TravelThreadViewer {
  const fullName =
    user.name?.trim() ||
    [user.firstName, user.lastName].filter(Boolean).join(" ").trim() ||
    null;
  return {
    user: {
      id: user.id,
      fullName,
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
 * Doit être notifié d’un message non lu : direction de l’établissement du séjour
 * (toutes les directions si Groupe scolaire), toute la compta, créateur.
 */
export function viewerIsTravelThreadAudience(
  trip: TravelThreadTripRef,
  viewer: TravelThreadViewer,
  establishments: Establishment[],
): boolean {
  if (isTripOwnerForThread(trip, viewer)) return true;
  if (viewerIsCompta(viewer.roles)) return true;
  const etab = trip.data?.etablissement;
  if (isGroupeScolaireRef(etab) && isAnyDirectionRole(viewer.roles)) return true;
  return userCanActAsDirectionFor(
    viewer.user,
    establishments,
    etab,
    viewer.roles,
    viewer.extraUserIds,
  );
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
