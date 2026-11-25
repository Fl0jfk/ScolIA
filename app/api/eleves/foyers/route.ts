import { NextResponse } from "next/server";
import { requireFoyersGate } from "@/app/lib/foyers-import/access";
import { listFoyersWithChildren } from "@/app/lib/foyers-import/service";

export const runtime = "nodejs";

/** Liste des foyers (responsables + enfants), lecture seule, mêmes droits que l'import. */
export async function GET(req: Request) {
  try {
    const gate = await requireFoyersGate();
    if (!gate.ok) return gate.response;
    const url = new URL(req.url);
    const out = await listFoyersWithChildren(gate.etablissementId, {
      q: url.searchParams.get("q") ?? "",
      limit: Number(url.searchParams.get("limit") ?? 50),
      offset: Number(url.searchParams.get("offset") ?? 0),
    });
    return NextResponse.json(out);
  } catch (err) {
    console.error("[eleves/foyers]", err);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}
