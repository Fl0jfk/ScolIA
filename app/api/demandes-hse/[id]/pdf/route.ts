import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  buildHseAcceptancePdf,
  hseAcceptancePdfFilename,
  type HseAcceptanceRecord,
} from "@/app/lib/hse-acceptance-pdf";
import { getJson } from "@/app/lib/s3-storage";
import { canViewHseDemand } from "@/app/lib/demandes-hse-access";

const INDEX_KEY = "demandes-hse/index.json";

type HseRecord = HseAcceptanceRecord & {
  status: string;
  createdBy: { userId: string; name: string; email: string };
  acceptancePdfPath?: string;
};


export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  const { id } = await ctx.params;
  if (!id?.trim()) {
    return NextResponse.json({ error: "Identifiant manquant." }, { status: 400 });
  }

  try {
    const hit = await getJson<HseRecord[]>(INDEX_KEY);
    const record = hit?.data?.find((r) => r.id === id);
    if (!record) {
      return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });
    }
    if (!canViewHseDemand(record, userId, roles)) {
      return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    }
    if (record.status !== "ACCEPTEE") {
      return NextResponse.json(
        { error: "L'attestation PDF n'est disponible que pour les demandes acceptées." },
        { status: 400 },
      );
    }

    const bytes = Buffer.from(await buildHseAcceptancePdf(record));

    const filename = hseAcceptancePdfFilename(record);
    return new NextResponse(Buffer.from(bytes), {
      status: 200,
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `attachment; filename="${filename}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (error) {
    console.error("[demandes-hse/pdf]", error);
    return NextResponse.json({ error: "Impossible de générer le PDF." }, { status: 500 });
  }
}
