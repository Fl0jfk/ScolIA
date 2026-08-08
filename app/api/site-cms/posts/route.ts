import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { isCustomWebsiteEnabled } from "@/app/lib/site-cms/access";
import { loadPostsIndex, savePost, slugify } from "@/app/lib/site-cms/storage";
import type { SitePost } from "@/app/lib/site-cms/types";

export const runtime = "nodejs";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  if (!(await isCustomWebsiteEnabled())) {
    return NextResponse.json(
      { error: "Site vitrine Scola non activé.", code: "CUSTOM_WEBSITE_OFF", items: [] },
      { status: 403 },
    );
  }
  const index = await loadPostsIndex();
  return NextResponse.json({ items: index });
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  if (!(await isCustomWebsiteEnabled())) {
    return NextResponse.json(
      { error: "Site vitrine Scola non activé.", code: "CUSTOM_WEBSITE_OFF" },
      { status: 403 },
    );
  }

  try {
    const body = await req.json();
    const title = String(body.title || "").trim();
    if (!title) return NextResponse.json({ error: "Titre requis" }, { status: 400 });
    const now = new Date().toISOString();
    const id = `post_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
    const status = body.status === "published" ? "published" : "draft";
    const post: SitePost = {
      id,
      slug: slugify(String(body.slug || title)),
      title,
      excerpt: String(body.excerpt || "").trim() || undefined,
      body: String(body.body || "").trim(),
      coverUrl: String(body.coverUrl || "").trim() || undefined,
      status,
      createdAt: now,
      updatedAt: now,
      publishedAt: status === "published" ? now : undefined,
    };
    await savePost(post);
    return NextResponse.json({ success: true, post });
  } catch (e) {
    console.error("[site-cms/posts POST]", e);
    return NextResponse.json({ error: "Création impossible" }, { status: 500 });
  }
}
