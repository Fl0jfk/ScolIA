import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantAdminRole } from "@/app/lib/intranet-auth";
import { findDbUserByExternalId } from "@/app/lib/members-db";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import { writeSecurityAudit } from "@/app/lib/security-audit";
import {
  listSessionsForAuthUserId,
  revokeAllSessionsForUser,
  revokeSessionByIdForUser,
  toPublicSession,
} from "@/app/lib/session-admin";

type Ctx = { params: Promise<{ externalUserId: string }> };

async function resolveTargetUser(externalUserIdRaw: string) {
  const externalUserId = decodeURIComponent(externalUserIdRaw).trim();
  if (!externalUserId) return null;
  const tenant = await getTenant();
  const etablissementId = await ensureEtablissementFromTenant(tenant);
  const dbUser = await findDbUserByExternalId(etablissementId, externalUserId);
  if (!dbUser) return null;
  return { dbUser, etablissementId, externalUserId };
}

/** Sessions d’un membre — réservé admin tenant / master. */
export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireTenantAdminRole();
  if (!gate.ok) return gate.response;

  const { externalUserId } = await ctx.params;
  const target = await resolveTargetUser(externalUserId);
  if (!target) {
    return NextResponse.json({ error: "Utilisateur introuvable." }, { status: 404 });
  }

  try {
    const rows = await listSessionsForAuthUserId(target.dbUser.id);
    return NextResponse.json({
      user: {
        id: target.dbUser.id,
        externalUserId: target.externalUserId,
        email: target.dbUser.email,
        name: target.dbUser.name,
      },
      sessions: rows.map((r) => toPublicSession(r, null)),
    });
  } catch (err) {
    console.error("[members/sessions GET]", err);
    return NextResponse.json(
      { error: "Impossible de charger les sessions." },
      { status: 500 },
    );
  }
}

export async function DELETE(req: Request, ctx: Ctx) {
  const gate = await requireTenantAdminRole();
  if (!gate.ok) return gate.response;
  const actor = await requireAppUser();
  const actorId = actor.ok ? actor.user.id : gate.ctx.userId;

  const { externalUserId } = await ctx.params;
  const target = await resolveTargetUser(externalUserId);
  if (!target) {
    return NextResponse.json({ error: "Utilisateur introuvable." }, { status: 404 });
  }

  try {
    const body = (await req.json().catch(() => ({}))) as {
      sessionId?: string;
      all?: boolean;
    };

    if (body.all === true) {
      const count = await revokeAllSessionsForUser(target.dbUser.id);
      await writeSecurityAudit({
        userId: actorId,
        action: "sessions_revoked_all_admin",
        req,
        metadata: {
          targetUserId: target.dbUser.id,
          targetExternalUserId: target.externalUserId,
          count,
        },
      });
      return NextResponse.json({ success: true, revoked: count });
    }

    const sessionId = String(body.sessionId || "").trim();
    if (!sessionId) {
      return NextResponse.json(
        { error: "sessionId ou all: true requis." },
        { status: 400 },
      );
    }

    const ok = await revokeSessionByIdForUser(target.dbUser.id, sessionId);
    if (!ok) {
      return NextResponse.json({ error: "Session introuvable." }, { status: 404 });
    }
    await writeSecurityAudit({
      userId: actorId,
      action: "session_revoked",
      req,
      metadata: {
        targetUserId: target.dbUser.id,
        targetExternalUserId: target.externalUserId,
        sessionId,
        admin: true,
      },
    });
    return NextResponse.json({ success: true, revoked: 1 });
  } catch (err) {
    console.error("[members/sessions DELETE]", err);
    return NextResponse.json(
      { error: "Impossible de révoquer la session." },
      { status: 500 },
    );
  }
}
