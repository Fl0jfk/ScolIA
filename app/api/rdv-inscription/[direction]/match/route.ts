import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { matchPublicRdvInscription } from "@/app/lib/rdv-inscription-service";

const matchLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

type Ctx = { params: Promise<{ direction: string }> };

/** Matching protégé : e-mail obligatoire, pool = foyer du contact uniquement. */
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

    const body = (await req.json()) as Record<string, unknown>;
    const parentEmail = String(body.parentEmail || "").trim().toLowerCase();
    if (parentEmail) {
      if (!(await matchLimiter.allow(`email:${parentEmail}`))) {
        return NextResponse.json(
          { error: "Trop de tentatives pour cet e-mail." },
          { status: 429 },
        );
      }
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
