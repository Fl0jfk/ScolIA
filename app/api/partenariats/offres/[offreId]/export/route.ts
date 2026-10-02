import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  getPartenariatOffreById,
  inscriptionsToCsv,
  listPartenariatInscriptions,
} from "@/app/lib/partenariats-db";

type Ctx = { params: Promise<{ offreId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const offre = await getPartenariatOffreById(offreId);
    if (!offre) return NextResponse.json({ error: "Offre introuvable." }, { status: 404 });
    const rows = await listPartenariatInscriptions(offreId);
    const csv = inscriptionsToCsv(rows);
    const filename = `inscriptions-${offre.slug}.csv`;
    return new NextResponse(csv, {
      status: 200,
      headers: {
        "Content-Type": "text/csv; charset=utf-8",
        "Content-Disposition": `attachment; filename="${filename}"`,
      },
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
