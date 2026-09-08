import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getEffectiveViewUser } from "@/app/lib/app-session";
import {
  resolveOcrCapabilitiesForUserServer,
  resolveOneDriveProfileForUserServer,
} from "@/app/lib/onedrive-user-profiles.server";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await getEffectiveViewUser();
  if (!user) return NextResponse.json({ profile: null, fluxes: [] });

  // businessUserId (métier / ex-Clerk) en priorité — expandOcrMatchIdsForUser
  // ajoute aussi l’id Better-Auth via e-mail / auth_user_mapping.
  const like = {
    id: user.businessUserId || user.externalUserId || user.id,
    fullName: user.name ?? null,
    lastName: user.lastName ?? null,
    firstName: user.firstName ?? null,
    emailAddresses: user.email ? [{ emailAddress: user.email }] : [],
    primaryEmailAddress: user.email ? { emailAddress: user.email } : null,
  };

  const caps = await resolveOcrCapabilitiesForUserServer(like);
  const profile = caps.primaryEleves ?? (await resolveOneDriveProfileForUserServer(like));
  return NextResponse.json({
    profile,
    fluxes: caps.fluxes.map((f) => ({
      id: f.id,
      kind: f.kind,
      label: f.label,
      basePath: f.basePath,
      secteur: f.secteur,
    })),
  });
}
