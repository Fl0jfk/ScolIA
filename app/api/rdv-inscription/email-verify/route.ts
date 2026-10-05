import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { verifyRdvEmailGateToken } from "@/app/lib/rdv-inscription-email-gate";
import { rdvInscriptionErrorRedirectPath } from "@/app/lib/rdv-inscription-service";
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

/** Valide le lien reçu par e-mail et pose le cookie de session gate. */
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
    const result = await verifyRdvEmailGateToken(token);
    if (!result.ok) {
      return NextResponse.redirect(
        await tenantAbsolutePath(
          rdvInscriptionErrorRedirectPath(result.directionSlug || hint, result.error),
        ),
      );
    }

    return NextResponse.redirect(await tenantAbsolutePath(result.redirectPath));
  } catch (e) {
    console.error("[rdv-inscription/email-verify]", e);
    const hint = directionHintFromRequest(req);
    return NextResponse.redirect(
      await tenantAbsolutePath(
        rdvInscriptionErrorRedirectPath(
          hint,
          "Lien de confirmation invalide. Demandez un nouvel e-mail.",
        ),
      ),
    );
  }
}
