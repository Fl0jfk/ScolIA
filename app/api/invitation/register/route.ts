import { NextResponse } from "next/server";
import { z } from "zod";
import { registerInvitationRsvp } from "@/app/lib/invitation-db";
import { sendInvitationRsvpConfirmation } from "@/app/lib/invitation-mail";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { INVITATION_DIPLOMAS, INVITATION_RESPONSES, INVITATION_SITUATION_STATUSES } from "@/app/lib/invitation-types";

const invitationLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 12,
});

const BodySchema = z.object({
  slug: z.string().min(1).max(80),
  eleveFirstName: z.string().min(1).max(80),
  eleveLastName: z.string().min(1).max(80),
  birthDate: z.string().max(32).optional().nullable(),
  response: z.enum(INVITATION_RESPONSES),
  presentCount: z.number().int().min(1).max(50).optional(),
  parentEmail: z.string().email().max(200),
  diploma: z.enum(INVITATION_DIPLOMAS).optional().nullable(),
  situationStatus: z.enum(INVITATION_SITUATION_STATUSES).optional().nullable(),
  situationDetail: z.string().max(300).optional(),
  situationEstablishment: z.string().max(200).optional(),
  website: z.string().optional(),
  company: z.string().optional(),
});

export async function POST(req: Request) {
  try {
    if (!(await invitationLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const raw = await req.json();
    const honeypot = String(raw.website || raw.company || "").trim();
    if (honeypot) {
      return NextResponse.json({ success: true });
    }

    const body = BodySchema.parse(raw);
    const result = await registerInvitationRsvp({
      slug: body.slug,
      eleveFirstName: body.eleveFirstName,
      eleveLastName: body.eleveLastName,
      birthDate: body.birthDate ?? null,
      response: body.response,
      presentCount: body.presentCount,
      parentEmail: body.parentEmail,
      diploma: body.diploma ?? null,
      situationStatus: body.situationStatus ?? null,
      situationDetail: body.situationDetail,
      situationEstablishment: body.situationEstablishment,
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    const { mailSent } = await sendInvitationRsvpConfirmation({
      page: result.page,
      rsvp: result.rsvp,
    });

    return NextResponse.json({
      success: true,
      updated: result.updated,
      rsvpId: result.rsvp.id,
      response: result.rsvp.response,
      presentCount: result.rsvp.presentCount,
      mailSent,
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
