import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { startRdvEmailGate } from "@/app/lib/rdv-inscription-email-gate";

const limiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 8,
});

type Ctx = { params: Promise<{ direction: string }> };

/** Envoie le lien de confirmation e-mail (gate avant le formulaire). */
export async function POST(req: Request, ctx: Ctx) {
  try {
    const ip = clientIpFromRequest(req);
    if (!(await limiter.allow(ip))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const { direction } = await ctx.params;
    const slug = decodeURIComponent(direction || "").trim().toLowerCase();
    const body = (await req.json()) as Record<string, unknown>;
    const parentEmail = String(body.parentEmail || "").trim().toLowerCase();
    if (parentEmail) {
      if (!(await limiter.allow(`email:${parentEmail}`))) {
        return NextResponse.json(
          { error: "Trop de tentatives pour cet e-mail." },
          { status: 429 },
        );
      }
    }

    const result = await startRdvEmailGate({ directionSlug: slug, parentEmail });
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }
    return NextResponse.json({
      pending: true,
      message:
        "Un e-mail vient de vous être envoyé. Cliquez sur le lien pour accéder au formulaire.",
      mailWarning: result.mailWarning,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
