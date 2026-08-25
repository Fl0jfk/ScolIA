import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { runOmogenWeeklySync } from "@/app/lib/nomenclature-import/omogen-sync";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const configured =
    process.env.OMOGEN_ENABLED?.trim() === "1" &&
    Boolean(process.env.OMOGEN_FETCH_URL?.trim()) &&
    Boolean(process.env.OMOGEN_CLIENT_CERT?.trim()) &&
    Boolean(process.env.OMOGEN_CLIENT_KEY?.trim());

  return NextResponse.json({
    configured,
    enabled: process.env.OMOGEN_ENABLED?.trim() === "1",
    fetchUrl: process.env.OMOGEN_FETCH_URL ? "défini" : null,
  });
}

export async function POST() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const result = await runOmogenWeeklySync(etabId);
  return NextResponse.json(result, { status: result.ok ? 200 : result.configured ? 502 : 503 });
}
