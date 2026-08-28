import { normalizeRequestEmail } from "@/app/lib/requests-board";
import { formatPersonName, isReservationBookedForOther } from "@/app/lib/prof-room-reservation-label";

export type RoomReservationOwnershipRow = {
  userId?: string;
  email?: string;
  bookedForOther?: boolean;
  bookedByUserId?: string;
  firstName?: string;
  lastName?: string;
  bookedByFirstName?: string;
  bookedByLastName?: string;
};

export type RoomReservationViewer = {
  userId: string;
  email?: string;
  firstName?: string;
  lastName?: string;
};

function normName(firstName?: string | null, lastName?: string | null): string {
  return formatPersonName(firstName, lastName).replace(/\s+/g, " ").toLowerCase();
}

function viewerMatchesName(
  viewer: RoomReservationViewer,
  firstName?: string | null,
  lastName?: string | null,
): boolean {
  const viewerName = normName(viewer.firstName, viewer.lastName);
  const targetName = normName(firstName, lastName);
  return Boolean(viewerName && targetName && viewerName === targetName);
}

/**
 * Indique si la réservation concerne le viewer en tant que bénéficiaire
 * (créneau « pour moi »), et non l’admin qui a réservé pour quelqu’un d’autre.
 */
export function isReservationBeneficiary(
  reservation: RoomReservationOwnershipRow,
  viewer: RoomReservationViewer,
): boolean {
  const emailNorm = normalizeRequestEmail(viewer.email || "");
  const forOther = isReservationBookedForOther(reservation);

  if (forOther) {
    if (reservation.bookedByUserId && reservation.bookedByUserId === viewer.userId) {
      return false;
    }

    if (!reservation.bookedByUserId && reservation.userId && reservation.userId === viewer.userId) {
      if (viewerMatchesName(viewer, reservation.bookedByFirstName, reservation.bookedByLastName)) {
        return false;
      }
      if (viewerMatchesName(viewer, reservation.firstName, reservation.lastName)) {
        return true;
      }
      return false;
    }

    if (reservation.userId && reservation.userId === viewer.userId) {
      return true;
    }

    if (emailNorm && normalizeRequestEmail(reservation.email || "") === emailNorm) {
      return true;
    }

    if (viewerMatchesName(viewer, reservation.firstName, reservation.lastName)) {
      return true;
    }

    return false;
  }

  if (reservation.userId && reservation.userId === viewer.userId) return true;
  if (emailNorm && normalizeRequestEmail(reservation.email || "") === emailNorm) return true;
  return false;
}

/** Alias historique pour les signaux dashboard (« ma salle »). */
export function isOwnRoomReservation(
  reservation: RoomReservationOwnershipRow,
  userId: string,
  emailNorm: string,
  viewerNames?: Pick<RoomReservationViewer, "firstName" | "lastName">,
): boolean {
  return isReservationBeneficiary(reservation, {
    userId,
    email: emailNorm,
    ...viewerNames,
  });
}
