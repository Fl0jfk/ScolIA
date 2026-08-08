import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { isCustomWebsiteEnabled } from "@/app/lib/site-cms/access";
import { deletePost, loadPost, savePost, slugify } from "@/app/lib/site-cms/storage";
import type { SitePost } from "@/app/lib/site-cms/types";

export const runtime = "nodejs";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  if (!(await isCustomWebsiteEnabled())) {
    return NextResponse.json({ error: "Site vitrine non activé" }, { status: 403 });
  }
  const { id } = await ctx.params;
  const post = await loadPost(id);
  if (!post) return NextResponse.json({ error: "Introuvable" }, { status: 404 });
  return NextResponse.json({ post });
}

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  if (!(await isCustomWebsiteEnabled())) {
    return NextResponse.json({ error: "Site vitrine non activé" }, { status: 403 });
  }

  try {
    const { id } = await ctx.params;
    const existing = await loadPost(id);
    if (!existing) return NextResponse.json({ error: "Introuvable" }, { status: 404 });
    const body = await req.json();
    const now = new Date().toISOString();
    const status =
      body.status === "published" || body.status === "draft" ? body.status : existing.status;
    const post: SitePost = {
      ...existing,
      title: body.title !== undefined ? String(body.title).trim() : existing.title,
      slug:
        body.slug !== undefined
          ? slugify(String(body.slug))
          : body.title !== undefined
            ? slugify(String(body.title))
            : existing.slug,
      excerpt:
        body.excerpt !== undefined
          ? String(body.excerpt).trim() || undefined
          : existing.excerpt,
      body: body.body !== undefined ? String(body.body) : existing.body,
      coverUrl:
        body.coverUrl !== undefined
          ? String(body.coverUrl).trim() || undefined
          : existing.coverUrl,
      status,
      updatedAt: now,
      publishedAt:
        status === "published"
          ? existing.publishedAt || now
          : undefined,
    };
    if (!post.title) return NextResponse.json({ error: "Titre requis" }, { status: 400 });
    await savePost(post);
    return NextResponse.json({ success: true, post });
  } catch (e) {
    console.error("[site-cms/posts PATCH]", e);
    return NextResponse.json({ error: "Mise à jour impossible" }, { status: 500 });
  }
}

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  if (!(await isCustomWebsiteEnabled())) {
    return NextResponse.json({ error: "Site vitrine non activé" }, { status: 403 });
  }
  const { id } = await ctx.params;
  const existing = await loadPost(id);
  if (!existing) return NextResponse.json({ error: "Introuvable" }, { status: 404 });
  await deletePost(id);
  return NextResponse.json({ success: true });
}
