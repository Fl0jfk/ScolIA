import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { matchPublicRdvInscription } from "@/app/lib/rdv-inscription-service";
import { readRdvEmailGateSession } from "@/app/lib/rdv-inscription-email-gate";
import { normalizeParentEmail } from "@/app/lib/eleves-parent-emails";

const matchLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

type Ctx = { params: Promise<{ direction: string }> };

/** Matching protégé : session e-mail vérifiée obligatoire. */
export async function POST(req: Request, ctx: Ctx) {
  try {
    const ip = clientIpFromRequest(req);
    if (!(await matchLimiter.allow(ip))) {
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

    const body = (await req.json()) as Record<string, unknown>;
    const parentEmail = normalizeParentEmail(String(body.parentEmail || ""));
    if (parentEmail !== gate.email) {
      return NextResponse.json(
        { error: "L’e-mail ne correspond pas à la session vérifiée." },
        { status: 403 },
      );
    }

    const result = await matchPublicRdvInscription({
      slug,
      parentEmail,
      parentPhone: String(body.parentPhone || "").trim(),
      studentFirstName: String(body.studentFirstName || "").trim(),
      studentLastName: String(body.studentLastName || "").trim(),
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    console.info(
      "[rdv-inscription/match]",
      JSON.stringify({
        ip,
        email: parentEmail,
        slug,
        count: result.candidates.length,
      }),
    );

    return NextResponse.json({
      candidates: result.candidates,
      homeEtablissement: result.homeEtablissement,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
