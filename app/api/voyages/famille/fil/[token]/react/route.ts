import { feedJson, withFeedFamily } from "@/app/lib/travels-feed-public-api";
import { reactAsFamily } from "@/app/lib/travels-feed-store";

type Params = { params: Promise<{ token: string }> };

/** Réaction d'une famille (👍 ❤️ 😄 👏) : une par type et par famille, retirable. Pas de commentaire possible. */
export async function POST(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withFeedFamily(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    const body = (await req.json().catch(() => null)) as { postId?: unknown; emoji?: unknown; on?: unknown } | null;
    const res = await reactAsFamily(g.ctx, { postId: body?.postId, emoji: body?.emoji, on: body?.on });
    if (!res.ok) return feedJson({ error: res.error }, res.status);
    return feedJson({ ok: true });
  } catch (e) {
    console.error("[voyages-fil] réaction", e instanceof Error ? e.message : "erreur");
    return feedJson({ error: "Réaction impossible." }, 500);
  }
}
