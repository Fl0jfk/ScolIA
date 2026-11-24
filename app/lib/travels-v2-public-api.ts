import "server-only";

import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { resolvePublicContext, type PublicContext } from "@/app/lib/travels-v2-registration-store";
import { tokenAccessFor, type TokenAccess } from "@/app/lib/travels-v2-registration-types";

/**
 * Garde commune des API publiques « page des familles » (sans compte).
 * Le jeton est vérifié CÔTÉ SERVEUR à chaque appel ; toute erreur de jeton donne la même réponse (aucun oracle).
 */

const ipLimiter = createMemoryRateLimiter({ name: "voyfam-ip", windowMs: 10 * 60 * 1000, max: 300 });
const badTokenLimiter = createMemoryRateLimiter({ name: "voyfam-bad", windowMs: 10 * 60 * 1000, max: 20 });
const invLimiter = createMemoryRateLimiter({ name: "voyfam-inv", windowMs: 60 * 60 * 1000, max: 400 });
export const codeIpLimiter = createMemoryRateLimiter({ name: "voyfam-code-ip", windowMs: 60 * 60 * 1000, max: 30 });
export const signLimiter = createMemoryRateLimiter({ name: "voyfam-sign", windowMs: 60 * 60 * 1000, max: 25 });
export const uploadLimiter = createMemoryRateLimiter({ name: "voyfam-upload", windowMs: 60 * 60 * 1000, max: 60 });
export const resendIpLimiter = createMemoryRateLimiter({ name: "voyfam-resend-ip", windowMs: 60 * 60 * 1000, max: 8 });
export const resendEmailLimiter = createMemoryRateLimiter({ name: "voyfam-resend-mail", windowMs: 60 * 60 * 1000, max: 3 });

export const NO_STORE = { "Cache-Control": "no-store", "Referrer-Policy": "no-referrer" } as const;

export function jsonNoStore(body: unknown, status = 200): NextResponse {
  return NextResponse.json(body, { status, headers: NO_STORE });
}

export const INVALID_LINK = "Lien invalide ou expiré.";

/** Refuse les écritures venant d'une autre origine (si l'en-tête Origin est présent). */
export function sameOriginOk(req: Request): boolean {
  const origin = req.headers.get("origin");
  if (!origin) return true;
  const host = req.headers.get("x-forwarded-host") || req.headers.get("host") || "";
  try {
    return new URL(origin).host === host;
  } catch {
    return false;
  }
}

export async function withPublicContext(
  req: Request,
  token: string,
  opts?: {
    write?: boolean;
    /** Route du seul PDF d'engagement : accepté même quand le voyage n'est plus ouvert (famille ayant signé « oui »). */
    allowPdfOnly?: boolean;
  },
): Promise<{ ok: true; ctx: PublicContext; access: TokenAccess } | { ok: false; response: NextResponse }> {
  const ip = clientIpFromRequest(req);
  if (!(await ipLimiter.allow(ip))) {
    return { ok: false, response: jsonNoStore({ error: "Trop de requêtes. Réessayez plus tard." }, 429) };
  }
  if (opts?.write && !sameOriginOk(req)) {
    return { ok: false, response: jsonNoStore({ error: "Requête refusée." }, 403) };
  }
  const ctx = await resolvePublicContext(token);
  if (!ctx) {
    if (!(await badTokenLimiter.allow(ip))) {
      return { ok: false, response: jsonNoStore({ error: "Trop de requêtes. Réessayez plus tard." }, 429) };
    }
    return { ok: false, response: jsonNoStore({ error: INVALID_LINK }, 404) };
  }
  if (!(await invLimiter.allow(ctx.inv.id))) {
    return { ok: false, response: jsonNoStore({ error: "Trop de requêtes. Réessayez plus tard." }, 429) };
  }
  // Un jeton ne donne plus rien dès que le voyage n'est plus ouvert, sauf à une famille qui a signé « oui » (voir tokenAccessFor).
  // Même réponse que pour un lien inconnu (aucun oracle).
  const access = tokenAccessFor({
    tripStatus: ctx.trip.status,
    signedYes: ctx.inv.signature?.decision === "oui",
    withdrawn: Boolean(ctx.inv.withdrawal),
  });
  const allowed = access === "full" || access === "signed_only" || (access === "pdf_only" && opts?.allowPdfOnly === true && !opts.write);
  if (!allowed) return { ok: false, response: jsonNoStore({ error: INVALID_LINK }, 404) };
  return { ok: true, ctx, access };
}
