import { jsonNoStore, withPublicContext } from "@/app/lib/travels-v2-public-api";
import { publicViewFor, saveAnswers } from "@/app/lib/travels-v2-registration-store";

type Params = { params: Promise<{ token: string }> };

/** Vue de la page pour CETTE invitation uniquement (jamais d'autre élève, jamais d'e-mail en clair). */
export async function GET(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token);
  if (!g.ok) return g.response;
  try {
    return jsonNoStore(await publicViewFor(g.ctx));
  } catch (e) {
    console.error("[voyages-famille] view", e);
    return jsonNoStore({ error: "Impossible de charger la page." }, 500);
  }
}

/** Enregistrement des réponses au fil de la saisie (brouillon). */
export async function PATCH(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    const body = await req.json().catch(() => null);
    const res = await saveAnswers(g.ctx, body);
    if (!res.ok) return jsonNoStore({ error: res.error }, res.status);
    return jsonNoStore(await publicViewFor(g.ctx));
  } catch (e) {
    console.error("[voyages-famille] answers", e);
    return jsonNoStore({ error: "Enregistrement impossible." }, 500);
  }
}
