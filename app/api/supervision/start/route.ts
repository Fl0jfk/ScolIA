import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { writeSecurityAudit } from "@/app/lib/security-audit";
import {
  canStartSupervision,
  loadSupervisionTargetProfile,
  sealSupervisionCookie,
  supervisionCookieSetOptions,
} from "@/app/lib/supervision";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { findDbUserByExternalId } from "@/app/lib/members-db";
import { isDatabaseConfigured } from "@/db/index";

/** Démarre une supervision lecture seule sur un collègue staff. */
export async function POST(req: Request) {
  const actor = await requireAppUser();
  if (!actor.ok) {
    return NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 });
  }
  if (!canStartSupervision(actor.user)) {
    return NextResponse.json(
      {
        error: "Réservé aux administrateurs et à la direction.",
        code: "SUPERVISION_FORBIDDEN",
      },
      { status: 403 },
    );
  }

  const tenantScope = await requireTenantId();
  if (!tenantScope.ok) return tenantScope.response;

  if (!isDatabaseConfigured()) {
    return NextResponse.json({ error: "Base de données requise." }, { status: 503 });
  }

  let body: { userId?: unknown; externalUserId?: unknown; email?: unknown };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "Corps JSON invalide." }, { status: 400 });
  }

  const etablissementId = tenantScope.ctx.etablissementId;
  let targetUserId = String(body.userId ?? "").trim();
  const externalUserId = String(body.externalUserId ?? "").trim();

  if (!targetUserId && externalUserId) {
    const row = await findDbUserByExternalId(etablissementId, externalUserId);
    targetUserId = row?.id ?? "";
  }

  if (!targetUserId) {
    const emailHint = String(body.email ?? externalUserId ?? "")
      .trim()
      .toLowerCase();
    if (emailHint.includes("@") && isDatabaseConfigured()) {
      const { getDb } = await import("@/db/index");
      const { user } = await import("@/db/schema");
      const { and, eq } = await import("drizzle-orm");
      const db = getDb();
      const [byEmail] = await db
        .select({ id: user.id })
        .from(user)
        .where(and(eq(user.etablissementId, etablissementId), eq(user.email, emailHint)))
        .limit(1);
      targetUserId = byEmail?.id ?? "";
    }
  }

  if (!targetUserId) {
    return NextResponse.json(
      { error: "Utilisateur cible requis (userId ou externalUserId)." },
      { status: 400 },
    );
  }

  if (targetUserId === actor.user.id) {
    return NextResponse.json(
      { error: "Vous ne pouvez pas superviser votre propre compte." },
      { status: 400 },
    );
  }

  const target = await loadSupervisionTargetProfile(targetUserId, etablissementId);
  if (!target) {
    return NextResponse.json(
      {
        error: "Cible introuvable ou non staff sur cet établissement.",
        code: "SUPERVISION_TARGET_INVALID",
      },
      { status: 404 },
    );
  }

  const sealed = sealSupervisionCookie({
    actorUserId: actor.user.id,
    targetUserId: target.userId,
    etablissementId,
    startedAt: Date.now(),
  });

  await writeSecurityAudit({
    userId: actor.user.id,
    action: "supervision_start",
    req,
    metadata: {
      targetUserId: target.userId,
      targetEmail: target.email,
      etablissementId,
    },
  });

  const res = NextResponse.json({
    ok: true,
    target: {
      userId: target.userId,
      email: target.email,
      displayName: target.displayName,
      roles: target.roles,
    },
  });
  res.cookies.set(supervisionCookieSetOptions(sealed));
  return res;
}
