import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  createPartenariatOffre,
  listPartenariatOffres,
} from "@/app/lib/partenariats-db";
import { PARTENARIAT_KINDS } from "@/app/lib/partenariats-types";

const CreateSchema = z.object({
  title: z.string().min(1).max(200),
  slug: z.string().max(80).optional(),
  shortDescription: z.string().max(500).optional(),
  body: z.string().max(20000).optional(),
  kind: z.enum(PARTENARIAT_KINDS).optional(),
  cycles: z.array(z.string()).optional(),
  niveaux: z.array(z.string()).optional(),
  categoryLabel: z.string().max(80).optional(),
  enabled: z.boolean().optional(),
});

export async function GET() {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const offres = await listPartenariatOffres();
    return NextResponse.json({ offres });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const body = CreateSchema.parse(await req.json());
    const offre = await createPartenariatOffre(body);
    return NextResponse.json({ offre }, { status: 201 });
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
