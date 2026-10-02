import { NextResponse } from "next/server";
import { getPublicPartenariatDetail } from "@/app/lib/partenariats-db";

type Ctx = { params: Promise<{ slug: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  try {
    const { slug } = await ctx.params;
    const offre = await getPublicPartenariatDetail(slug);
    if (!offre) {
      return NextResponse.json({ error: "Offre introuvable." }, { status: 404 });
    }
    return NextResponse.json({ offre });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
