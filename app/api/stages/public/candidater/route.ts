import { NextResponse } from "next/server";
import { resolveOfferByCandidatureToken, submitOfferCandidature } from "@/app/lib/stage-candidature";
import { listOfferApplications } from "@/app/lib/stage-storage";
import { STAGE_OFFER_KIND_LABELS } from "@/app/lib/stage-types";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const candidaterLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

export async function GET(req: Request) {
  try {
    const token = new URL(req.url).searchParams.get("token")?.trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const offer = await resolveOfferByCandidatureToken(token);
    if (!offer) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });
    if (offer.status !== "approved") {
      return NextResponse.json({ error: "Cette offre n'accepte plus de candidatures." }, { status: 403 });
    }

    const applications = await listOfferApplications(offer.id);
    const placesLeft = Math.max(0, offer.positionsCount - applications.length);

    return NextResponse.json({
      offer: {
        id: offer.id,
        kind: offer.kind,
        kindLabel: STAGE_OFFER_KIND_LABELS[offer.kind],
        companyName: offer.companyName,
        description: offer.description,
        targetLevels: offer.targetLevels,
        periodStart: offer.periodStart,
        periodEnd: offer.periodEnd,
        positionsCount: offer.positionsCount,
        placesLeft,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    if (!(await candidaterLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const token = String(body.token ?? "").trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const result = await submitOfferCandidature({
      token,
      student: {
        firstName: String(body.firstName ?? "").trim(),
        lastName: String(body.lastName ?? "").trim(),
        className: String(body.className ?? "").trim(),
        level: String(body.level ?? "").trim(),
        email: String(body.email ?? "").trim() || undefined,
        parentEmail: String(body.parentEmail ?? "").trim() || undefined,
      },
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }

    return NextResponse.json({
      success: true,
      studentLink: result.studentLink,
      conventionId: result.convention.id,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
