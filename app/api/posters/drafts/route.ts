import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  MAX_POSTER_DRAFTS,
  formatLabel,
  loadPosterDraftsIndex,
  parsePosterDraft,
  savePosterDraft,
} from "@/app/lib/posters";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const items = await loadPosterDraftsIndex();
  return NextResponse.json({
    max: MAX_POSTER_DRAFTS,
    items: items.map((e) => ({
      ...e,
      formatLabel: formatLabel(e.format),
    })),
  });
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const draft = parsePosterDraft(body.draft ?? body);
    const id = typeof body.id === "string" ? body.id : undefined;
    const user = await safeCurrentUser();

    const stored = await savePosterDraft({
      id,
      draft,
      createdBy: {
        userId: user?.id || gate.ctx.userId,
        name: user?.fullName || undefined,
        email: user?.primaryEmailAddress?.emailAddress || undefined,
      },
    });

    const items = await loadPosterDraftsIndex();
    return NextResponse.json({
      success: true,
      draft: {
        id: stored.id,
        title: stored.title,
        updatedAt: stored.updatedAt,
        createdAt: stored.createdAt,
        format: stored.format,
        formatLabel: formatLabel(stored.format),
      },
      items: items.map((e) => ({
        ...e,
        formatLabel: formatLabel(e.format),
      })),
      max: MAX_POSTER_DRAFTS,
    });
  } catch (e) {
    console.error("[posters/drafts POST]", e);
    const msg = e instanceof Error ? e.message : "Enregistrement impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
