import { NextResponse } from "next/server";
import {
  archiveScoliaConversation,
  getScoliaConversation,
  renameScoliaConversation,
} from "@/app/lib/brain-ai/scolia-conversations-db";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { isDatabaseConfigured } from "@/db/index";

export const runtime = "nodejs";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  try {
    const user = await safeCurrentUser();
    if (!user) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    if (isEleveBienEtreProfile(rolesFromUserLike(user))) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    if (!isDatabaseConfigured()) {
      return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
    }
    const tenant = await requireTenantId();
    if (!tenant.ok) return tenant.response;
    const { id } = await ctx.params;
    const conversation = await getScoliaConversation({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      conversationId: id,
    });
    if (!conversation) {
      return NextResponse.json({ error: "Conversation introuvable." }, { status: 404 });
    }
    return NextResponse.json({ conversation });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function PATCH(req: Request, ctx: Ctx) {
  try {
    const user = await safeCurrentUser();
    if (!user) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    if (!isDatabaseConfigured()) {
      return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
    }
    const tenant = await requireTenantId();
    if (!tenant.ok) return tenant.response;
    const { id } = await ctx.params;
    const body = (await req.json()) as { title?: string; archive?: boolean };
    if (body.archive) {
      const ok = await archiveScoliaConversation({
        etablissementId: tenant.ctx.etablissementId,
        userId: tenant.ctx.authUserId,
        conversationId: id,
      });
      return NextResponse.json({ success: ok });
    }
    if (body.title) {
      const ok = await renameScoliaConversation({
        etablissementId: tenant.ctx.etablissementId,
        userId: tenant.ctx.authUserId,
        conversationId: id,
        title: body.title,
      });
      return NextResponse.json({ success: ok });
    }
    return NextResponse.json({ error: "Rien à mettre à jour." }, { status: 400 });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
