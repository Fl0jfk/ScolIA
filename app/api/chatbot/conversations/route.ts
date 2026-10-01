import { NextResponse } from "next/server";
import {
  createScoliaConversation,
  listScoliaConversations,
} from "@/app/lib/brain-ai/scolia-conversations-db";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { isDatabaseConfigured } from "@/db/index";

export const runtime = "nodejs";

export async function GET() {
  try {
    const user = await safeCurrentUser();
    if (!user) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    }
    if (isEleveBienEtreProfile(rolesFromUserLike(user))) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    if (!isDatabaseConfigured()) {
      return NextResponse.json({ conversations: [] });
    }
    const tenant = await requireTenantId();
    if (!tenant.ok) return tenant.response;

    const conversations = await listScoliaConversations({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      limit: 50,
    });
    return NextResponse.json({ conversations });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const user = await safeCurrentUser();
    if (!user) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    }
    if (isEleveBienEtreProfile(rolesFromUserLike(user))) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    if (!isDatabaseConfigured()) {
      return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
    }
    const tenant = await requireTenantId();
    if (!tenant.ok) return tenant.response;

    const body = (await req.json().catch(() => ({}))) as {
      title?: string;
      conversationId?: string;
    };
    const created = await createScoliaConversation({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      title: body.title,
      conversationId: body.conversationId,
    });
    return NextResponse.json({ conversation: created });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
