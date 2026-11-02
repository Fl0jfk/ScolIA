import { after, NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import { registerInvitationRsvpAsStaff } from "@/app/lib/invitation-db";
import { sendInvitationRsvpConfirmation } from "@/app/lib/invitation-mail";
import {
  INVITATION_DIPLOMAS,
  INVITATION_RESPONSES,
  INVITATION_SITUATION_STATUSES,
} from "@/app/lib/invitation-types";

type Ctx = { params: Promise<{ pageId: string }> };

const BodySchema = z
  .object({
    eligibleId: z.string().uuid().optional().nullable(),
    eleveFirstName: z.string().max(80).optional(),
    eleveLastName: z.string().max(80).optional(),
    birthDate: z.string().max(32).optional().nullable(),
    response: z.enum(INVITATION_RESPONSES),
    presentCount: z.number().int().min(1).max(50).optional(),
    parentEmail: z.string().email().max(200),
    diploma: z.enum(INVITATION_DIPLOMAS).optional().nullable(),
    situationStatus: z.enum(INVITATION_SITUATION_STATUSES).optional().nullable(),
    situationDetail: z.string().max(300).optional(),
    situationEstablishment: z.string().max(200).optional(),
    /** Si false : n’envoie pas le mail de confirmation à la famille. Défaut : true. */
    sendMail: z.boolean().optional(),
  })
  .superRefine((val, ctx) => {
    if (val.eligibleId) return;
    if (!val.eleveFirstName?.trim() || !val.eleveLastName?.trim()) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Sélectionnez un élève de la liste, ou saisissez prénom et nom.",
        path: ["eleveFirstName"],
      });
    }
    if (!val.birthDate?.trim()) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Date de naissance requise si l’élève n’est pas pris dans la liste.",
        path: ["birthDate"],
      });
    }
  });

/** Personnel : enregistrer / modifier une réponse à la place d’une famille. */
export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const raw = await req.json();
    const body = BodySchema.parse(raw);
    const result = await registerInvitationRsvpAsStaff(pageId, {
      eligibleId: body.eligibleId ?? null,
      eleveFirstName: body.eleveFirstName,
      eleveLastName: body.eleveLastName,
      birthDate: body.birthDate,
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

    const sendMail = body.sendMail !== false;
    if (sendMail) {
      after(() => {
        void sendInvitationRsvpConfirmation({
          page: result.page,
          rsvp: result.rsvp,
        }).catch((err: unknown) => {
          console.error(
            "[invitation] mail confirmation (staff)",
            err instanceof Error ? err.message : err,
          );
        });
      });
    }

    return NextResponse.json({
      success: true,
      updated: result.updated,
      rsvp: result.rsvp,
      mailQueued: sendMail,
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
