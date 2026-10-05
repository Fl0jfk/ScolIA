import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  deletePartenariatOffre,
  getPartenariatOffreById,
  listPartenariatEvenements,
  listPartenariatInscriptions,
  updatePartenariatOffre,
} from "@/app/lib/partenariats-db";
import { PARTENARIAT_KINDS } from "@/app/lib/partenariats-types";

const CtaSchema = z.object({
  label: z.string().min(1).max(120),
  url: z.string().min(1).max(2000),
});

const TarifSchema = z.object({
  label: z.string().min(1).max(200),
  amountCents: z.number().int().nullable().optional(),
  amountLabel: z.string().max(80).optional(),
  note: z.string().max(500).optional(),
});

const PatchSchema = z.object({
  title: z.string().min(1).max(200).optional(),
  slug: z.string().max(80).optional(),
  shortDescription: z.string().max(500).optional(),
  body: z.string().max(20000).optional(),
  logoS3Key: z.string().max(500).nullable().optional(),
  kind: z.enum(PARTENARIAT_KINDS).optional(),
  cycles: z.array(z.string()).optional(),
  niveaux: z.array(z.string()).optional(),
  categoryLabel: z.string().max(80).optional(),
  contactName: z.string().max(120).optional(),
  contactRole: z.string().max(120).optional(),
  contactEmail: z.string().max(200).optional(),
  contactPhone: z.string().max(40).optional(),
  partnerContactName: z.string().max(120).optional(),
  partnerContactRole: z.string().max(120).optional(),
  partnerContactEmail: z.string().max(200).optional(),
  partnerContactPhone: z.string().max(40).optional(),
  ctaLinks: z.array(CtaSchema).optional(),
  tarifs: z.array(TarifSchema).optional(),
  demarche: z.string().max(8000).optional(),
  engagementText: z.string().max(2000).optional(),
  requireSignature: z.boolean().optional(),
  notifyEmail: z.string().max(200).nullable().optional(),
  maxPlaces: z.number().int().nullable().optional(),
  inscriptionOpensAt: z.string().nullable().optional(),
  inscriptionClosesAt: z.string().nullable().optional(),
  enabled: z.boolean().optional(),
  sortOrder: z.number().int().optional(),
});

type Ctx = { params: Promise<{ offreId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const offre = await getPartenariatOffreById(offreId);
    if (!offre) return NextResponse.json({ error: "Offre introuvable." }, { status: 404 });
    const [evenements, inscriptions] = await Promise.all([
      listPartenariatEvenements(offreId),
      listPartenariatInscriptions(offreId),
    ]);
    return NextResponse.json({ offre, evenements, inscriptions });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const body = PatchSchema.parse(await req.json());
    const offre = await updatePartenariatOffre(offreId, body);
    return NextResponse.json({ offre });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json(
        { error: e.issues[0]?.message || "Données invalides." },
        { status: 400 },
      );
    }
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("introuvable")) {
      return NextResponse.json({ error: msg }, { status: 404 });
    }
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    await deletePartenariatOffre(offreId);
    return NextResponse.json({ success: true });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
