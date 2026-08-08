import { NextResponse } from "next/server";
import { isCustomWebsiteEnabled, getCustomWebsiteDomain } from "@/app/lib/site-cms/access";
import { listPublishedPosts } from "@/app/lib/site-cms/storage";

export const runtime = "nodejs";

/** Articles publiés pour la vitrine — sans auth. */
export async function GET() {
  try {
    if (!(await isCustomWebsiteEnabled())) {
      return NextResponse.json({ posts: [], enabled: false });
    }
    const posts = await listPublishedPosts();
    const domain = await getCustomWebsiteDomain();
    return NextResponse.json({
      enabled: true,
      primaryDomain: domain || null,
      posts: posts.map((p) => ({
        id: p.id,
        slug: p.slug,
        title: p.title,
        excerpt: p.excerpt || null,
        body: p.body,
        coverUrl: p.coverUrl || null,
        publishedAt: p.publishedAt || p.updatedAt,
        updatedAt: p.updatedAt,
      })),
    });
  } catch (e) {
    console.error("[public/site/posts]", e);
    return NextResponse.json({ error: "Indisponible" }, { status: 500 });
  }
}
