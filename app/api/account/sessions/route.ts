import { NextResponse } from "next/server";
import { headers } from "next/headers";
import { requireAppUser } from "@/app/lib/app-session";
import { getBetterAuth } from "@/app/lib/auth-server";
import { writeSecurityAudit } from "@/app/lib/security-audit";
import {
  listSessionsForAuthUserId,
  revokeOtherSessionsForUser,
  revokeSessionByIdForUser,
  toPublicSession,
} from "@/app/lib/session-admin";

/** Liste / révocation des sessions du compte connecté. */
export async function GET() {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  try {
    const hdrs = await headers();
    const current = await getBetterAuth().api.getSession({ headers: hdrs });
    const currentId = current?.session?.id ?? null;
    const rows = await listSessionsForAuthUserId(appUser.user.id);
    return NextResponse.json({
      sessions: rows.map((r) => toPublicSession(r, currentId)),
    });
  } catch (err) {
    console.error("[account/sessions GET]", err);
    return NextResponse.json(
      { error: "Impossible de charger les sessions." },
      { status: 500 },
    );
  }
}

export async function DELETE(req: Request) {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  try {
    const body = (await req.json().catch(() => ({}))) as {
      sessionId?: string;
      others?: boolean;
    };
    const hdrs = await headers();
    const current = await getBetterAuth().api.getSession({ headers: hdrs });
    const currentId = current?.session?.id ?? null;

    if (body.others === true) {
      if (!currentId) {
        return NextResponse.json(
          { error: "Session courante introuvable." },
          { status: 400 },
        );
      }
      const count = await revokeOtherSessionsForUser(appUser.user.id, currentId);
      await writeSecurityAudit({
        userId: appUser.user.id,
        action: "sessions_revoked_others",
        req,
        metadata: { count },
      });
      return NextResponse.json({ success: true, revoked: count });
    }

    const sessionId = String(body.sessionId || "").trim();
    if (!sessionId) {
      return NextResponse.json({ error: "sessionId requis." }, { status: 400 });
    }

    const ok = await revokeSessionByIdForUser(appUser.user.id, sessionId);
    if (!ok) {
      return NextResponse.json({ error: "Session introuvable." }, { status: 404 });
    }
    await writeSecurityAudit({
      userId: appUser.user.id,
      action: "session_revoked",
      req,
      metadata: { sessionId, self: sessionId === currentId },
    });

    return NextResponse.json({
      success: true,
      revokedCurrent: sessionId === currentId,
    });
  } catch (err) {
    console.error("[account/sessions DELETE]", err);
    return NextResponse.json(
      { error: "Impossible de révoquer la session." },
      { status: 500 },
    );
  }
}
