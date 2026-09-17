import { NextRequest, NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { confirmPublicRdvInscription } from "@/app/lib/rdv-inscription-service";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

const confirmLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

async function redirectToConfirme(query: Record<string, string>) {
  const base = await tenantAbsolutePath("/rdv-inscription/confirme");
  const u = new URL(base);
  for (const [k, v] of Object.entries(query)) u.searchParams.set(k, v);
  return NextResponse.redirect(u);
}

/** Validation du créneau via lien e-mail (double opt-in). */
export async function GET(req: NextRequest) {
  try {
    if (!(await confirmLimiter.allow(clientIpFromRequest(req)))) {
      return redirectToConfirme({ erreur: "rate" });
    }

    const token = req.nextUrl.searchParams.get("token")?.trim() || "";
    if (!token) {
      return redirectToConfirme({ erreur: "lien_invalide" });
    }

    const result = await confirmPublicRdvInscription(token);
    if (!result.ok) {
      if (result.error === "expired") {
        return redirectToConfirme({ erreur: "expire" });
      }
      if (result.error === "taken") {
        return redirectToConfirme({ erreur: "pris" });
      }
      return redirectToConfirme({ erreur: "lien_invalide" });
    }

    return redirectToConfirme({
      ok: "1",
      start: result.booking.startAt,
      end: result.booking.endAt,
      ...(result.already ? { deja: "1" } : {}),
    });
  } catch (e) {
    console.error("[rdv-inscription] confirm", e);
    return redirectToConfirme({ erreur: "erreur" });
  }
}
