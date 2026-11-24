import { clientIpFromRequest } from "@/app/lib/memory-rate-limit";
import { codeIpLimiter, jsonNoStore, withPublicContext } from "@/app/lib/travels-v2-public-api";
import { sendSigningCode } from "@/app/lib/travels-v2-registration-store";

type Params = { params: Promise<{ token: string }> };

/** « Recevoir mon code » : code à 6 chiffres envoyé à l'e-mail de l'invitation (jamais à une adresse saisie). */
export async function POST(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    if (!(await codeIpLimiter.allow(clientIpFromRequest(req)))) {
      return jsonNoStore({ error: "Trop de demandes de code. Réessayez plus tard." }, 429);
    }
    const res = await sendSigningCode(g.ctx);
    if (!res.ok) return jsonNoStore({ error: res.error, retryAfterSec: res.retryAfterSec }, res.status);
    return jsonNoStore({ ok: true, maskedEmail: res.maskedEmail });
  } catch (e) {
    console.error("[voyages-famille] code", e);
    return jsonNoStore({ error: "Envoi du code impossible." }, 500);
  }
}
