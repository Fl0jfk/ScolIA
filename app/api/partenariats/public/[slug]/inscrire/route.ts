import { NextResponse } from "next/server";
import { z } from "zod";
import {
  getPartenariatOffreBySlug,
  submitPartenariatInscription,
} from "@/app/lib/partenariats-db";
import { notifyPartenariatInscription } from "@/app/lib/partenariats-mail";

const BodySchema = z.object({
  eleveFirstName: z.string().min(1).max(80),
  eleveLastName: z.string().min(1).max(80),
  eleveNiveau: z.string().max(40).optional(),
  eleveClasse: z.string().max(40).optional(),
  eleveBirthDate: z.string().max(10).nullable().optional(),
  parentFirstName: z.string().min(1).max(80),
  parentLastName: z.string().min(1).max(80),
  parentEmail: z.string().email().max(200),
  parentPhone: z.string().max(40).optional(),
  engagementAccepted: z.boolean(),
  signatureDataUrl: z.string().max(3_000_000).nullable().optional(),
});

type Ctx = { params: Promise<{ slug: string }> };

/** Rate-limit mémoire très simple (par IP + slug). */
const recent = new Map<string, number>();
const WINDOW_MS = 15_000;

export async function POST(req: Request, ctx: Ctx) {
  try {
    const { slug } = await ctx.params;
    const ip =
      req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
      req.headers.get("x-real-ip") ||
      "unknown";
    const key = `${ip}:${slug}`;
    const now = Date.now();
    const last = recent.get(key) || 0;
    if (now - last < WINDOW_MS) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques secondes." },
        { status: 429 },
      );
    }
    recent.set(key, now);

    const body = BodySchema.parse(await req.json());
    const inscription = await submitPartenariatInscription(slug, body);
    const offre = await getPartenariatOffreBySlug(slug);
    if (offre) {
      void notifyPartenariatInscription({ offre, inscription });
    }
    return NextResponse.json({ success: true, inscriptionId: inscription.id }, { status: 201 });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json(
        { error: e.issues[0]?.message || "Données invalides." },
        { status: 400 },
      );
    }
    const msg = e instanceof Error ? e.message : String(e);
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
