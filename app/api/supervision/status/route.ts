import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { resolveActiveSupervision } from "@/app/lib/supervision";

/** État du mode supervision pour le bandeau UI. */
export async function GET() {
  const actor = await requireAppUser();
  if (!actor.ok) {
    return NextResponse.json({ active: false });
  }

  const active = await resolveActiveSupervision({ actorUserId: actor.user.id });
  if (!active) {
    return NextResponse.json({ active: false });
  }

  return NextResponse.json({
    active: true,
    target: {
      userId: active.target.userId,
      email: active.target.email,
      displayName: active.target.displayName,
      roles: active.target.roles,
    },
    startedAt: active.cookie.startedAt,
  });
}
