import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { resolveOneDriveProfileForClerkUserServer } from "@/app/lib/onedrive-user-profiles.server";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  if (!user) return NextResponse.json({ profile: null });
  const profile = await resolveOneDriveProfileForClerkUserServer(user);
  return NextResponse.json({ profile });
}
