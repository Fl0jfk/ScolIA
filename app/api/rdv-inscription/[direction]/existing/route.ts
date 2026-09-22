import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { getActiveRdvBookingForEleve } from "@/app/lib/rdv-inscription-service";
import { readRdvEmailGateSession } from "@/app/lib/rdv-inscription-email-gate";
import { normalizeParentEmail } from "@/app/lib/eleves-parent-emails";

const existingLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

type Ctx = { params: Promise<{ direction: string }> };

/** RDV déjà actif pour l’élève (session e-mail vérifiée). */
export async function GET(req: Request, ctx: Ctx) {
  try {
    if (!(await existingLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const { direction } = await ctx.params;
    const slug = decodeURIComponent(direction || "").trim().toLowerCase();
    if (!slug) {
      return NextResponse.json({ error: "Direction requise." }, { status: 400 });
    }

    const gate = await readRdvEmailGateSession({ directionSlug: slug });
    if (!gate) {
      return NextResponse.json(
        { error: "Confirmez d’abord votre e-mail via le lien reçu." },
        { status: 401 },
      );
    }

    const url = new URL(req.url);
    const eleveId = String(url.searchParams.get("eleveId") || "").trim();
    const parentEmail = normalizeParentEmail(
      String(url.searchParams.get("parentEmail") || gate.email),
    );
    if (parentEmail !== gate.email) {
      return NextResponse.json(
        { error: "L’e-mail ne correspond pas à la session vérifiée." },
        { status: 403 },
      );
    }
    if (!eleveId) {
      return NextResponse.json({ error: "eleveId requis." }, { status: 400 });
    }

    const result = await getActiveRdvBookingForEleve({
      directionSlug: slug,
      eleveId,
      parentEmail,
    });
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json({
      booking: result.booking
        ? {
            id: result.booking.id,
            status: result.booking.status,
            startAt: result.booking.startAt,
            endAt: result.booking.endAt,
            studentFirstName: result.booking.studentFirstName,
            studentLastName: result.booking.studentLastName,
            niveauLabel: result.booking.niveauLabel,
            createdAt: result.booking.createdAt,
            confirmedAt: result.booking.confirmedAt,
          }
        : null,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
