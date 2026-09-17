import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { bookPublicRdvInscription } from "@/app/lib/rdv-inscription-service";

const bookLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 10,
});

type Ctx = { params: Promise<{ direction: string }> };

/** Réservation publique d’un créneau d’inscription. */
export async function POST(req: Request, ctx: Ctx) {
  try {
    if (!(await bookLimiter.allow(clientIpFromRequest(req)))) {
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

    const body = (await req.json()) as Record<string, unknown>;
    const honeypot = String(body.website || body.company || "").trim();
    if (honeypot) {
      return NextResponse.json({ success: true });
    }

    const consent = body.consent === true || body.consent === "true" || body.consent === 1;
    if (!consent) {
      return NextResponse.json(
        { error: "Le consentement est requis pour prendre rendez-vous." },
        { status: 400 },
      );
    }

    const result = await bookPublicRdvInscription(slug, {
      eventId: String(body.eventId || "").trim(),
      studentFirstName: String(body.studentFirstName || "").trim(),
      studentLastName: String(body.studentLastName || "").trim(),
      parentEmail: String(body.parentEmail || "").trim(),
      parentPhone: String(body.parentPhone || "").trim(),
      niveauId: String(body.niveauId || "").trim(),
      eleveId: body.eleveId ? String(body.eleveId).trim() : null,
      createNew: body.createNew === true || body.createNew === "true" || body.createNew === 1,
      hasPap: body.hasPap === "yes" || body.hasPap === "no" ? body.hasPap : undefined,
      papS3Key: body.papS3Key ? String(body.papS3Key).trim() : null,
      papFileName: body.papFileName ? String(body.papFileName).trim() : null,
      papMimeType: body.papMimeType ? String(body.papMimeType).trim() : null,
      papBringToRdv:
        body.papBringToRdv === true ||
        body.papBringToRdv === "true" ||
        body.papBringToRdv === 1,
      etablissementOrigineRne: body.etablissementOrigineRne
        ? String(body.etablissementOrigineRne).trim()
        : null,
      etablissementOrigineLabel: body.etablissementOrigineLabel
        ? String(body.etablissementOrigineLabel).trim()
        : null,
      etablissementOrigineAdresse: body.etablissementOrigineAdresse
        ? String(body.etablissementOrigineAdresse).trim()
        : null,
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json({
      success: true,
      pending: true,
      bookingId: result.booking.id,
      startAt: result.booking.startAt,
      endAt: result.booking.endAt,
      mailWarning: result.mailWarning || undefined,
      message:
        "Un e-mail vient de vous être envoyé : cliquez sur le lien pour valider votre créneau.",
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
