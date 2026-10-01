import { NextResponse } from "next/server";
import { appendScoliaMessages } from "@/app/lib/brain-ai/scolia-conversations-db";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { isDatabaseConfigured } from "@/db/index";

export const runtime = "nodejs";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: Ctx) {
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
    const body = (await req.json()) as {
      messages?: Array<{ role: "user" | "assistant"; content: string }>;
      state?: Record<string, unknown> | null;
      titleHint?: string;
    };
    const messages = Array.isArray(body.messages) ? body.messages.slice(0, 20) : [];
    if (messages.length === 0) {
      return NextResponse.json({ error: "messages requis" }, { status: 400 });
    }
    const result = await appendScoliaMessages({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      conversationId: id,
      messages: messages.map((m) => ({
        role: m.role === "assistant" ? "assistant" : "user",
        content: String(m.content || "").slice(0, 20_000),
      })),
      state: body.state ?? null,
      titleHint: body.titleHint,
    });
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }
    return NextResponse.json({ success: true });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
