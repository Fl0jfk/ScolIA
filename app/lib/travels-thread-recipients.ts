import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  travelThreadViewerFromStaff,
  viewerIsTravelThreadAudience,
  type TravelThreadTripRef,
} from "@/app/lib/travels-thread-unread";

export type TravelThreadMemberRef = {
  userId?: string;
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles: string[];
  pending: boolean;
};

export type TravelThreadEmailRecipient = { email: string; name: string };

/**
 * Destinataires du mail « nouveau message » : même public que le compteur de non-lus
 * (créateur, compta, direction concernée), sans l’auteur du message, une seule fois par adresse.
 */
export function selectTravelThreadEmailRecipients(params: {
  trip: TravelThreadTripRef & { ownerEmail?: string | null };
  members: TravelThreadMemberRef[];
  establishments: Establishment[];
  authorUserIds: string[];
  authorEmail?: string | null;
}): TravelThreadEmailRecipient[] {
  const authorIds = new Set(params.authorUserIds.map((id) => id.trim()).filter(Boolean));
  const authorEmail = normalizeEmail(params.authorEmail);
  const seen = new Set<string>();
  const out: TravelThreadEmailRecipient[] = [];

  const push = (emailRaw: string | null | undefined, name: string) => {
    const email = normalizeEmail(emailRaw);
    if (!email || !email.includes("@")) return;
    if (email === authorEmail || seen.has(email)) return;
    seen.add(email);
    out.push({ email: String(emailRaw).trim(), name });
  };

  for (const m of params.members) {
    if (m.pending) continue;
    const ids = [m.userId, m.externalUserId].map((v) => String(v || "").trim()).filter(Boolean);
    if (ids.some((id) => authorIds.has(id))) continue;
    const viewer = travelThreadViewerFromStaff({
      id: m.userId || m.externalUserId,
      externalUserId: m.externalUserId,
      firstName: m.firstName,
      lastName: m.lastName,
      name: m.displayName,
      email: m.email,
      roles: m.roles,
    });
    if (!viewerIsTravelThreadAudience(params.trip, viewer, params.establishments)) continue;
    push(m.email, viewer.user?.fullName || m.email);
  }

  // Créateur absent de l’annuaire (compte supprimé, import ancien) : on se rabat sur l’e-mail du dossier.
  const ownerId = String(params.trip.ownerId || "").trim();
  if (!ownerId || !authorIds.has(ownerId)) {
    push(params.trip.ownerEmail, String(params.trip.ownerName || "").trim());
  }

  return out;
}

function normalizeEmail(email: string | null | undefined): string {
  return String(email || "").trim().toLowerCase();
}
