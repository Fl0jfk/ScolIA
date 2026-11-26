import "server-only";

import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { resolveFeedFamilyContext, type FeedFamilyContext } from "@/app/lib/travels-feed-store";

/**
 * Garde commune des API « fil du voyage » destinées aux familles (lien personnel, sans compte).
 * Le jeton est vérifié CÔTÉ SERVEUR à chaque appel ; toute cause de refus donne la MÊME réponse (aucun oracle).
 */
const ipLimiter = createMemoryRateLimiter({ name: "voyfeed-ip", windowMs: 10 * 60 * 1000, max: 240 });
const badTokenLimiter = createMemoryRateLimiter({ name: "voyfeed-bad", windowMs: 10 * 60 * 1000, max: 20 });
const tokenLimiter = createMemoryRateLimiter({ name: "voyfeed-tok", windowMs: 60 * 60 * 1000, max: 600 });
const reactLimiter = createMemoryRateLimiter({ name: "voyfeed-react", windowMs: 60 * 60 * 1000, max: 200 });

export const FEED_NO_STORE = {
  "Cache-Control": "no-store",
  "Referrer-Policy": "no-referrer",
  "X-Robots-Tag": "noindex, nofollow",
} as const;

export const FEED_INVALID_LINK = "Lien invalide ou expiré.";

export function feedJson(body: unknown, status = 200): NextResponse {
  return NextResponse.json(body, { status, headers: FEED_NO_STORE });
}

function sameOriginOk(req: Request): boolean {
  const origin = req.headers.get("origin");
  if (!origin) return true;
  const host = req.headers.get("x-forwarded-host") || req.headers.get("host") || "";
  try {
    return new URL(origin).host === host;
  } catch {
    return false;
  }
}

export async function withFeedFamily(
  req: Request,
  token: string,
  opts?: { write?: boolean },
): Promise<{ ok: true; ctx: FeedFamilyContext } | { ok: false; response: NextResponse }> {
  const ip = clientIpFromRequest(req);
  if (!(await ipLimiter.allow(ip))) return { ok: false, response: feedJson({ error: "Trop de requêtes. Réessayez plus tard." }, 429) };
  if (opts?.write && !sameOriginOk(req)) return { ok: false, response: feedJson({ error: "Requête refusée." }, 403) };
  const ctx = await resolveFeedFamilyContext(token);
  if (!ctx) {
    if (!(await badTokenLimiter.allow(ip))) return { ok: false, response: feedJson({ error: "Trop de requêtes. Réessayez plus tard." }, 429) };
    return { ok: false, response: feedJson({ error: FEED_INVALID_LINK }, 404) };
  }
  if (!(await tokenLimiter.allow(ctx.access.id))) return { ok: false, response: feedJson({ error: "Trop de requêtes. Réessayez plus tard." }, 429) };
  if (opts?.write && !(await reactLimiter.allow(ctx.access.id))) return { ok: false, response: feedJson({ error: "Trop de réactions. Réessayez plus tard." }, 429) };
  return { ok: true, ctx };
}
