import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  getPosterTemplateMeta,
  parsePosterDraft,
  posterGeneratedFileKey,
  posterTitleFromDraft,
  renderPosterPdf,
  saveGeneratedPoster,
} from "@/app/lib/posters";

export const runtime = "nodejs";
export const maxDuration = 60;

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const draft = parsePosterDraft(body.draft ?? body);
    const meta = getPosterTemplateMeta(draft.templateId);
    if (!meta) {
      return NextResponse.json({ error: "Modèle inconnu" }, { status: 400 });
    }

    const bytes = await renderPosterPdf(draft);
    const user = await safeCurrentUser();
    const id = `poster_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
    const title = posterTitleFromDraft(draft);

    const stored = await saveGeneratedPoster(
      {
        id,
        templateId: draft.templateId,
        templateLabel: meta.label,
        title,
        createdAt: new Date().toISOString(),
        createdBy: {
          userId: user?.id || gate.ctx.userId,
          name: user?.fullName || undefined,
          email: user?.primaryEmailAddress?.emailAddress || undefined,
        },
        fileKey: posterGeneratedFileKey(id),
        format: draft.format,
        draft,
      },
      bytes,
    );

    return NextResponse.json({
      success: true,
      poster: {
        id: stored.id,
        title: stored.title,
        templateId: stored.templateId,
        templateLabel: stored.templateLabel,
        createdAt: stored.createdAt,
        format: stored.format,
        downloadUrl: `/api/posters/generated/${stored.id}/pdf`,
      },
    });
  } catch (e) {
    console.error("[posters/generate]", e);
    const msg = e instanceof Error ? e.message : "Génération impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
