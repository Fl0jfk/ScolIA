import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { writeSecurityAudit } from "@/app/lib/security-audit";
import {
  readSupervisionCookieFromJar,
  supervisionCookieClearOptions,
} from "@/app/lib/supervision";

/** Quitte le mode supervision lecture seule. */
export async function POST(req: Request) {
  const actor = await requireAppUser();
  if (!actor.ok) {
    return NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 });
  }

  const cookie = await readSupervisionCookieFromJar();
  if (cookie && cookie.actorUserId === actor.user.id) {
    await writeSecurityAudit({
      userId: actor.user.id,
      action: "supervision_stop",
      req,
      metadata: {
        targetUserId: cookie.targetUserId,
        etablissementId: cookie.etablissementId,
      },
    });
  }

  const res = NextResponse.json({ ok: true });
  res.cookies.set(supervisionCookieClearOptions());
  return res;
}
