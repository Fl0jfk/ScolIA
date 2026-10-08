import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { uniqueClassesByLevelFromEleves } from "@/app/lib/class-allocation-level-heuristic";
import { CLASS_LEVELS, type ClassLevel } from "@/app/lib/class-allocation-types";
import { loadCampaignConfig, saveCampaignConfig } from "@/app/lib/class-allocation-storage";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const body = (await req.json().catch(() => ({}))) as { level?: ClassLevel };
  const levelFilter = body.level && CLASS_LEVELS.includes(body.level) ? body.level : null;

  const eleves = await loadElevesActifsRegistry();
  const byLevel = uniqueClassesByLevelFromEleves(eleves);
  const current = await loadCampaignConfig();
  const levelMap = new Map(current.levels.map((l) => [l.level, { ...l }]));

  for (const level of CLASS_LEVELS) {
    if (levelFilter && level !== levelFilter) continue;
    const classes = byLevel[level];
    if (!classes.length) continue;
    const existing = levelMap.get(level) || { level, sourceClassPrefixes: [], targetClasses: [] };
    levelMap.set(level, { ...existing, sourceClassPrefixes: classes });
  }

  const levels = Array.from(levelMap.values()).filter(
    (l) => l.sourceClassPrefixes.length > 0 || l.targetClasses.length > 0,
  );
  const config = { ...current, levels };
  await saveCampaignConfig(config);

  const filled = levelFilter
    ? byLevel[levelFilter].length
    : CLASS_LEVELS.reduce((n, l) => n + byLevel[l].length, 0);

  return NextResponse.json({
    ok: true,
    config,
    message: filled
      ? `${filled} classe(s) actuelle(s) chargée(s) depuis la liste élèves.`
      : "Aucune classe reconnue dans la liste élèves (vérifiez les libellés CP, 6e, 2nde…).",
  });
}
