import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  resolveOcrCapabilitiesForUserServer,
  resolveOneDriveProfileForUserServer,
} from "@/app/lib/onedrive-user-profiles.server";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  if (!user) return NextResponse.json({ profile: null, fluxes: [] });
  const like = {
    id: user.id,
    lastName: user.lastName,
    emailAddresses: user.emailAddresses?.map((e) => ({ emailAddress: e.emailAddress })),
    primaryEmailAddress: user.primaryEmailAddress
      ? { emailAddress: user.primaryEmailAddress.emailAddress }
      : null,
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
