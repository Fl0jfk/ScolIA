import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  deletePosterDraft,
  formatLabel,
  loadPosterDraft,
  loadPosterDraftsIndex,
  resolvePosterDraftAssetUrls,
} from "@/app/lib/posters";

export const runtime = "nodejs";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  const stored = await loadPosterDraft(id);
  if (!stored?.draft) {
    return NextResponse.json({ error: "Brouillon introuvable" }, { status: 404 });
  }

  const assets = await resolvePosterDraftAssetUrls(stored.draft);
  return NextResponse.json({
    success: true,
    draft: {
      id: stored.id,
      title: stored.title,
      updatedAt: stored.updatedAt,
      createdAt: stored.createdAt,
      format: stored.format,
      formatLabel: formatLabel(stored.format),
      data: stored.draft,
    },
    assets,
  });
}

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  await deletePosterDraft(id);
  const items = await loadPosterDraftsIndex();
  return NextResponse.json({
    success: true,
    items: items.map((e) => ({
      ...e,
      formatLabel: formatLabel(e.format),
    })),
  });
}
