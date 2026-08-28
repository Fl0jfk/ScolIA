import { NextResponse } from "next/server";
import { createPublicPreconventionDraft } from "@/app/lib/stage-workflow";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const preconventionLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

export async function POST(req: Request) {
  try {
    if (!(await preconventionLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const firstName = String(body.firstName ?? "").trim();
    const lastName = String(body.lastName ?? "").trim();
    const className = String(body.className ?? "").trim();
    const level = String(body.level ?? "3e").trim();

    if (!firstName || !lastName || !className) {
      return NextResponse.json(
        { error: "Prénom, nom et classe sont obligatoires." },
        { status: 400 },
      );
    }

    const { convention, studentLink } = await createPublicPreconventionDraft({
      firstName,
      lastName,
      className,
      level,
    });

    return NextResponse.json({
      success: true,
      conventionId: convention.id,
      studentLink,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
