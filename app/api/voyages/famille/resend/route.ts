import { after } from "next/server";
import { z } from "zod";
import { clientIpFromRequest } from "@/app/lib/memory-rate-limit";
import { normalizeParentEmail } from "@/app/lib/eleves-parent-emails";
import { getTenant, getTenantAppUrl, runWithTenant } from "@/app/lib/tenant-context";
import {
  jsonNoStore,
  resendEmailLimiter,
  resendIpLimiter,
  sameOriginOk,
} from "@/app/lib/travels-v2-public-api";
import { sha256Hex } from "@/app/lib/travels-v2-registration";
import { resendLinksForEmail } from "@/app/lib/travels-v2-registration-store";

const bodySchema = z.object({ email: z.string().trim().min(3).max(200) }).strict();

const SAME_ANSWER = {
  ok: true,
  message: "Si cette adresse est connue, un e-mail contenant votre lien vient d'être envoyé.",
};

/**
 * Renvoi du lien. Réponse IDENTIQUE que l'adresse soit connue ou non (aucune fuite) ; le travail se fait après la
 * réponse pour ne pas laisser de différence de délai. Limites par IP et par adresse.
 */
export async function POST(req: Request) {
  if (!sameOriginOk(req)) return jsonNoStore({ error: "Requête refusée." }, 403);
  const parsed = bodySchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return jsonNoStore({ error: "Adresse e-mail invalide." }, 400);
  const email = normalizeParentEmail(parsed.data.email);
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return jsonNoStore({ error: "Adresse e-mail invalide." }, 400);

  if (!(await resendIpLimiter.allow(clientIpFromRequest(req)))) {
    return jsonNoStore({ error: "Trop de demandes. Réessayez plus tard." }, 429);
  }
  // Au-delà de la limite par adresse : on répond pareil mais on n'envoie plus rien.
  const allowedForEmail = await resendEmailLimiter.allow(sha256Hex(`resend:${email}`));

  if (allowedForEmail) {
    const tenant = await getTenant();
    const baseUrl = await getTenantAppUrl();
    after(async () => {
      try {
        await runWithTenant(tenant, () => resendLinksForEmail(email, baseUrl));
      } catch (e) {
        console.error("[voyages-famille] resend", e instanceof Error ? e.message : e);
      }
    });
  }
  return jsonNoStore(SAME_ANSWER);
}
