import { jsonNoStore, signLimiter, withPublicContext } from "@/app/lib/travels-v2-public-api";
import { publicViewFor } from "@/app/lib/travels-v2-registration-store";
import { submitObjection } from "@/app/lib/travels-v2-lot3-store";

type Params = { params: Promise<{ token: string }> };

/** « Je ne suis pas d'accord » : désaccord du second responsable, confirmé par le code reçu par e-mail. */
export async function POST(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    if (!(await signLimiter.allow(g.ctx.inv.id))) {
      return jsonNoStore({ error: "Trop de tentatives. Réessayez plus tard." }, 429);
    }
    const res = await submitObjection(g.ctx, await req.json().catch(() => null));
    if (!res.ok) return jsonNoStore({ error: res.error, attemptsLeft: res.attemptsLeft }, res.status);
    return jsonNoStore({ ok: true, view: await publicViewFor(g.ctx) });
  } catch (e) {
    console.error("[voyages-famille] objection", e);
    return jsonNoStore({ error: "Enregistrement impossible. Réessayez." }, 500);
  }
}
