import { feedJson, withFeedFamily } from "@/app/lib/travels-feed-public-api";
import { buildFamilyFeedView } from "@/app/lib/travels-feed-store";
import { REACTION_EMOJIS } from "@/app/lib/travels-feed-logic";

type Params = { params: Promise<{ token: string }> };

/** Fil privé d'un voyage : lecture seule pour les familles (liens des photos pré-signés ≤ 10 min, générés à chaque appel). */
export async function GET(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withFeedFamily(req, token);
  if (!g.ok) return g.response;
  try {
    return feedJson(await buildFamilyFeedView(g.ctx, REACTION_EMOJIS));
  } catch (e) {
    console.error("[voyages-fil] lecture", e instanceof Error ? e.message : "erreur");
    return feedJson({ error: "Chargement impossible." }, 500);
  }
}
