import { NextResponse } from "next/server";
import { z } from "zod";
import { lookupInvitationIdentity } from "@/app/lib/invitation-db";

const BodySchema = z.object({
  slug: z.string().min(1).max(80),
  eleveFirstName: z.string().min(1).max(80),
  eleveLastName: z.string().min(1).max(80),
  birthDate: z.string().min(1).max(32),
});

export async function POST(req: Request) {
  try {
    const body = BodySchema.parse(await req.json());
    const result = await lookupInvitationIdentity({
      slug: body.slug,
      eleveFirstName: body.eleveFirstName,
      eleveLastName: body.eleveLastName,
      birthDate: body.birthDate,
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
    const raw = e instanceof Error ? e.message : String(e);
    const error =
      raw.startsWith("read tcp") || /i\/o timeout/i.test(raw)
        ? "Service momentanément indisponible. Réessayez dans un instant."
        : raw;
    return NextResponse.json({ error }, { status: 500 });
  }
}
