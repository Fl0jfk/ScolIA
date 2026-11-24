import { jsonNoStore, signLimiter, withPublicContext } from "@/app/lib/travels-v2-public-api";
import { publicViewFor, submitSignature } from "@/app/lib/travels-v2-registration-store";

type Params = { params: Promise<{ token: string }> };

/** Signature : nom + code + tracé (« oui »). Le « non » est confirmé par le code, sans tracé. */
export async function POST(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    if (!(await signLimiter.allow(g.ctx.inv.id))) {
      return jsonNoStore({ error: "Trop de tentatives. Réessayez plus tard." }, 429);
    }
    const body = await req.json().catch(() => null);
    const res = await submitSignature(g.ctx, body);
    if (!res.ok) return jsonNoStore({ error: res.error, attemptsLeft: res.attemptsLeft }, res.status);
    return jsonNoStore({ ok: true, decision: res.decision, view: await publicViewFor(g.ctx) });
  } catch (e) {
    console.error("[voyages-famille] sign", e);
    return jsonNoStore({ error: "Signature impossible. Réessayez." }, 500);
  }
}
