import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import {
  getParentBlogByToken,
  isParentBlogPublicExpired,
  listParentBlogPosts,
  loadTripTitleForPublic,
  purgeParentBlogContent,
} from "@/app/lib/travels-parent-blog";

const SECURE_HEADERS = {
  "X-Robots-Tag": "noindex, nofollow",
  "Cache-Control": "no-store",
  "Referrer-Policy": "no-referrer",
};
const publicBlogLimiter = createMemoryRateLimiter({ windowMs: 60_000, max: 60, name: "public-blog" });

function json(body: unknown, status = 200) {
  return NextResponse.json(body, { status, headers: SECURE_HEADERS });
}

/**
 * Lecture publique du blog parents (sans auth).
 * Aucune donnée élève / contact — titre, dates, messages et photos uniquement.
 */
export async function GET(req: Request) {
  try {
    if (!(await publicBlogLimiter.allow(`ip:${clientIpFromRequest(req)}`))) {
      return json({ error: "Trop de requêtes. Réessayez plus tard." }, 429);
    }
    const token = new URL(req.url).searchParams.get("token")?.trim() || "";
    if (!token) return json({ error: "Jeton manquant." }, 400);

    const blog = await getParentBlogByToken(token);
    if (!blog || !blog.enabled) {
      return json({ error: "Page introuvable." }, 404);
    }

    if (isParentBlogPublicExpired(blog.expiresAt)) {
      void purgeParentBlogContent(blog.etablissementId, blog.travelId).catch((err) =>
        console.error("[public-blog] purge", err),
      );
      return json(
        {
          error: "Cette page de suivi est fermée (délai de 7 jours après le retour écoulé).",
          expired: true,
        },
        410,
      );
    }

    const tripMeta = await loadTripTitleForPublic(blog.etablissementId, blog.travelId);
    if (!tripMeta) {
      return json({ error: "Page introuvable." }, 404);
    }

    const posts = await listParentBlogPosts(blog.etablissementId, blog.travelId, {
      includeSignedUrls: true,
    });

    return json({
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
    return json({ error: "Impossible de charger la page." }, 500);
  }
}
