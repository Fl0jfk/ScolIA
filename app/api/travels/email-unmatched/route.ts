import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { listUnmatchedEmailItems } from "@/app/lib/travel-email-ingest";

/** Liste les e-mails transport non rattachés automatiquement à un séjour. */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const items = await listUnmatchedEmailItems();
    return NextResponse.json({ items, count: items.length });
  } catch (e) {
    console.error("[email-unmatched]", e);
    return NextResponse.json({ error: "Erreur lecture unmatched" }, { status: 500 });
  }
}
