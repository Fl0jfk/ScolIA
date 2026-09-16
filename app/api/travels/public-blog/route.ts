import { NextResponse } from "next/server";
import {
  getParentBlogByToken,
  isParentBlogPublicExpired,
  listParentBlogPosts,
  loadTripTitleForPublic,
  purgeParentBlogContent,
} from "@/app/lib/travels-parent-blog";

/**
 * Lecture publique du blog parents (sans auth).
 * Aucune donnée élève / contact — titre, dates, messages et photos uniquement.
 */
export async function GET(req: Request) {
  try {
    const token = new URL(req.url).searchParams.get("token")?.trim() || "";
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const blog = await getParentBlogByToken(token);
    if (!blog || !blog.enabled) {
      return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    }

    if (isParentBlogPublicExpired(blog.expiresAt)) {
      void purgeParentBlogContent(blog.etablissementId, blog.travelId).catch((err) =>
        console.error("[public-blog] purge", err),
      );
      return NextResponse.json(
        {
          error: "Cette page de suivi est fermée (délai de 15 jours après le retour écoulé).",
          expired: true,
        },
        { status: 410 },
      );
    }

    const tripMeta = await loadTripTitleForPublic(blog.etablissementId, blog.travelId);
    if (!tripMeta) {
      return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    }

    const posts = await listParentBlogPosts(blog.etablissementId, blog.travelId, {
      includeSignedUrls: true,
    });

    return NextResponse.json({
      title: tripMeta.title,
      destination: tripMeta.destination,
      startDate: tripMeta.startDate,
      endDate: tripMeta.endDate,
      expiresAt: blog.expiresAt.toISOString(),
      posts: posts.map((p) => ({
        id: p.id,
        authorName: p.authorName,
        body: p.body,
        createdAt: p.createdAt,
        photos: p.photos.filter((ph) => ph.url).map((ph) => ({ id: ph.id, url: ph.url })),
      })),
    });
  } catch (e) {
    console.error("[public-blog]", e);
    return NextResponse.json({ error: "Impossible de charger la page." }, { status: 500 });
  }
}
