import "server-only";

import { resolveMemberProfileById } from "@/app/lib/members-db";
import { canReassignTravelsOwner } from "@/app/lib/travels-roles";
import type { AppActor } from "@/app/lib/app-actor-types";

type TravelsOwnerProfile = {
  ownerId: string;
  ownerName: string;
  ownerEmail: string;
};

async function resolveTravelsOwnerFromDirectory(
  externalUserId: string,
): Promise<TravelsOwnerProfile | null> {
  const id = externalUserId.trim();
  if (!id) return null;
  try {
    const profile = await resolveMemberProfileById(id);
    if (!profile?.email) return null;
    return {
      ownerId: profile.id,
      ownerName: profile.name || profile.email,
      ownerEmail: profile.email,
    };
  } catch {
    return null;
  }
}

/** Applique ownerId / ownerName / ownerEmail avec contrôle administratif si tiers. */
export async function applyTravelsOwnerAssignment(
  objectToSave: Record<string, unknown>,
  actor: AppActor | null | undefined,
  existingTrip?: Record<string, unknown> | null,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const actorId = actor?.id?.trim() || "";
  const existingOwnerId = String(existingTrip?.ownerId || "").trim();
  const requestedOwnerId = String(objectToSave.ownerId || "").trim();

  if (existingOwnerId) {
    const adminTransfers =
      requestedOwnerId &&
      requestedOwnerId !== existingOwnerId &&
      canReassignTravelsOwner(actor);

    if (adminTransfers) {
      const profile = await resolveTravelsOwnerFromDirectory(requestedOwnerId);
      if (!profile) {
        return {
          ok: false,
          status: 400,
          error: "Utilisateur introuvable ou sans adresse e-mail.",
        };
      }
      objectToSave.ownerId = profile.ownerId;
      objectToSave.ownerName = profile.ownerName;
      objectToSave.ownerEmail = profile.ownerEmail;
      return { ok: true };
    }

    objectToSave.ownerId = existingOwnerId;
    objectToSave.ownerName = existingTrip?.ownerName || objectToSave.ownerName || "Enseignant";
    objectToSave.ownerEmail = existingTrip?.ownerEmail || objectToSave.ownerEmail || "";
    return { ok: true };
  }

  if (!requestedOwnerId) {
    if (actorId) {
      objectToSave.ownerId = actorId;
      if (!objectToSave.ownerName) objectToSave.ownerName = actor?.fullName || "Enseignant";
      if (!objectToSave.ownerEmail) {
        objectToSave.ownerEmail = actor?.primaryEmailAddress?.emailAddress || "";
      }
    }
    return { ok: true };
  }

  if (requestedOwnerId === actorId) {
    const profile = await resolveTravelsOwnerFromDirectory(requestedOwnerId);
    if (profile) {
      objectToSave.ownerId = profile.ownerId;
      objectToSave.ownerName = profile.ownerName;
      objectToSave.ownerEmail = profile.ownerEmail;
    }
    return { ok: true };
  }

  if (!canReassignTravelsOwner(actor)) {
    return {
      ok: false,
      status: 403,
      error: "Seul l'administratif peut créer ou transférer un dossier pour un autre utilisateur.",
    };
  }

  const profile = await resolveTravelsOwnerFromDirectory(requestedOwnerId);
  if (!profile) {
    return {
      ok: false,
      status: 400,
      error: "Utilisateur introuvable ou sans adresse e-mail.",
    };
  }

  objectToSave.ownerId = profile.ownerId;
  objectToSave.ownerName = profile.ownerName;
  objectToSave.ownerEmail = profile.ownerEmail;
  return { ok: true };
}
