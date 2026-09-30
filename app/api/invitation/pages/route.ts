import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  createInvitationPage,
  listInvitationPages,
} from "@/app/lib/invitation-db";
import {
  INVITATION_DIPLOMA_MODES,
  INVITATION_THEMES,
} from "@/app/lib/invitation-types";

const CreateSchema = z.object({
  title: z.string().min(1).max(200),
  slug: z.string().max(80).optional(),
  intro: z.string().max(4000).optional(),
  theme: z.enum(INVITATION_THEMES).optional(),
  enabled: z.boolean().optional(),
  startsAt: z.string().nullable().optional(),
  endsAt: z.string().nullable().optional(),
  location: z.string().max(500).optional(),
  diplomaMode: z.enum(INVITATION_DIPLOMA_MODES).optional(),
  maxTotalPersons: z.number().int().min(1).max(50000).optional(),
  maxPersonsPerEleve: z.number().int().min(1).max(50).optional(),
  notifyEmail: z.string().max(200).nullable().optional(),
});

export async function GET() {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const pages = await listInvitationPages();
    return NextResponse.json({ pages });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const body = CreateSchema.parse(await req.json());
    const page = await createInvitationPage(body);
    return NextResponse.json({ page }, { status: 201 });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json({ error: e.issues[0]?.message || "Données invalides." }, { status: 400 });
    }
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
