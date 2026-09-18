import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { verifyRdvEmailGateToken } from "@/app/lib/rdv-inscription-email-gate";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

const limiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

/** Valide le lien reçu par e-mail et pose le cookie de session gate. */
export async function GET(req: Request) {
  try {
    const ip = clientIpFromRequest(req);
    if (!(await limiter.allow(ip))) {
      return NextResponse.redirect(
        await tenantAbsolutePath("/rdv-inscription/lycee?email_error=rate"),
      );
    }

    const token = new URL(req.url).searchParams.get("token") || "";
    const result = await verifyRdvEmailGateToken(token);
    if (!result.ok) {
      const slug = result.directionSlug || "lycee";
      return NextResponse.redirect(
        await tenantAbsolutePath(
          `/rdv-inscription/${encodeURIComponent(slug)}?email_error=${encodeURIComponent(result.error)}`,
        ),
      );
    }

    return NextResponse.redirect(await tenantAbsolutePath(result.redirectPath));
  } catch (e) {
    console.error("[rdv-inscription/email-verify]", e);
    return NextResponse.redirect(
      await tenantAbsolutePath("/rdv-inscription/lycee?email_error=error"),
    );
  }
}
