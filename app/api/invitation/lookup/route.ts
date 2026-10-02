import { NextResponse } from "next/server";
import { z } from "zod";
import { lookupInvitationIdentity } from "@/app/lib/invitation-db";

const BodySchema = z.object({
  slug: z.string().min(1).max(80),
  eleveFirstName: z.string().min(1).max(80),
  eleveLastName: z.string().min(1).max(80),
  birthDate: z.string().max(32).optional().nullable(),
});

export async function POST(req: Request) {
  try {
    const body = BodySchema.parse(await req.json());
    const result = await lookupInvitationIdentity({
      slug: body.slug,
      eleveFirstName: body.eleveFirstName,
      eleveLastName: body.eleveLastName,
      birthDate: body.birthDate ?? null,
    });
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }
    return NextResponse.json({
      ok: true,
      existing: result.existing
        ? {
            response: result.existing.response,
            presentCount: result.existing.presentCount,
            parentEmail: result.existing.parentEmail,
            diploma: result.existing.diploma,
            situationStatus: result.existing.situationStatus,
            situationDetail: result.existing.situationDetail,
            situationEstablishment: result.existing.situationEstablishment,
          }
        : null,
      matchScore: result.matchScore,
    });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json(
        { error: e.issues[0]?.message || "Données invalides." },
        { status: 400 },
      );
    }
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
