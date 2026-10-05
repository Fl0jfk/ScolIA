import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { setRdvEmailGateCookie } from "@/app/lib/rdv-inscription-email-gate";
import {
  consumeRdvRescheduleToken,
  rdvInscriptionErrorRedirectPath,
} from "@/app/lib/rdv-inscription-service";
import { isValidDirectionSlug } from "@/app/lib/rdv-inscription-types";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

const limiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

function directionHintFromRequest(req: Request): string | undefined {
  const raw = new URL(req.url).searchParams.get("direction") || "";
  const slug = raw.trim().toLowerCase();
  return slug && isValidDirectionSlug(slug) ? slug : undefined;
}

/**
 * Lien mail « choisir un autre créneau » :
 * pose le cookie gate (Route Handler) puis redirige vers la bonne direction.
 */
export async function GET(req: Request) {
  try {
    const ip = clientIpFromRequest(req);
    const hint = directionHintFromRequest(req);
    if (!(await limiter.allow(ip))) {
      return NextResponse.redirect(
        await tenantAbsolutePath(
          rdvInscriptionErrorRedirectPath(hint, "Trop de tentatives. Réessayez dans quelques minutes."),
        ),
      );
    }

    const token = new URL(req.url).searchParams.get("token") || "";
    const result = await consumeRdvRescheduleToken(token);
    if (!result.ok) {
      return NextResponse.redirect(
        await tenantAbsolutePath(
          rdvInscriptionErrorRedirectPath(result.directionSlug || hint, result.error),
        ),
      );
    }

    await setRdvEmailGateCookie(result.gateSession);
    return NextResponse.redirect(await tenantAbsolutePath(result.redirectPath));
  } catch (e) {
    console.error("[rdv-inscription/rebook]", e);
    const hint = directionHintFromRequest(req);
    return NextResponse.redirect(
      await tenantAbsolutePath(
        rdvInscriptionErrorRedirectPath(
          hint,
          "Lien de rechoix indisponible. Contactez l’établissement.",
        ),
      ),
    );
  }
}
