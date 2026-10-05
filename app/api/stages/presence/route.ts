import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { listStagePresenceOnDate } from "@/app/lib/occupancy/stages-read";
import { occupancy } from "@/app/lib/occupancy";

export async function GET(req: Request) {
  const gate = await requireModule("stages");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const date =
    new URL(req.url).searchParams.get("date")?.trim().slice(0, 10) || calendarDateKeyParis();

  try {
    const { rows, unmatched } = await listStagePresenceOnDate({
      etablissementId: etabId,
      date,
    });
    const presence = await occupancy({
      etablissementId: etabId,
      date,
      eleveIds: rows.map((r) => r.eleveId),
    });
    const tagByEleve = new Map(presence.facts.map((f) => [f.eleveId, f.tag]));

    return NextResponse.json({
      date,
      unmatched,
      eleves: rows.map((r) => ({
        ...r,
        occupancyTag: tagByEleve.get(r.eleveId) ?? "en_stage",
        excludeFromBulletin: true,
      })),
      resume: {
        total: rows.length,
        unmatched,
      },
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Chargement impossible." },
      { status: 400 },
    );
  }
}
