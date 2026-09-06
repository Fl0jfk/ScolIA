import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import { getTenantAppUrl } from "@/app/lib/tenant-context";
import {
  getToolboxConfigResolved,
  parseToolboxConfig,
  resolveToolboxRentreePages,
  saveToolboxConfig,
} from "@/app/lib/toolbox-config";
import { listPortesOuvertesRegistrations, countRegistrationsBySlotAndCycle } from "@/app/lib/portes-ouvertes-storage";
import { PORTES_OUVERTES_CYCLES, PORTES_OUVERTES_CYCLE_LABELS } from "@/app/lib/portes-ouvertes-types";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  try {
    const [config, app, publicOrigin] = await Promise.all([
      getToolboxConfigResolved(),
      loadAppConfig(),
      getTenantAppUrl(),
    ]);
    const registrations = await listPortesOuvertesRegistrations();
    const bySlotCycle = countRegistrationsBySlotAndCycle(registrations);
    const portesOuvertesStats: Record<string, number> = {};
    const portesOuvertesStatsByCycle: Record<
      string,
      Partial<Record<"ecole" | "college" | "lycee", number>>
    > = {};
    for (const slot of config.tools["portes-ouvertes"].slots) {
      const byCycle = bySlotCycle[slot.id] || {};
      portesOuvertesStatsByCycle[slot.id] = byCycle;
      portesOuvertesStats[slot.id] = PORTES_OUVERTES_CYCLES.reduce(
        (sum, c) => sum + (byCycle[c] || 0),
        0,
      );
    }
    return NextResponse.json({
      config,
      publicOrigin,
      establishments: app.establishments.filter((e) => e.active !== false),
      portesOuvertesStats,
      portesOuvertesStatsByCycle,
      cycleLabels: PORTES_OUVERTES_CYCLE_LABELS,
      registrationsCount: registrations.length,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const parsed = parseToolboxConfig(body);
    const app = await loadAppConfig();
    const resolved = resolveToolboxRentreePages(parsed, app.establishments);
    await saveToolboxConfig(resolved);
    return NextResponse.json({ success: true, config: resolved });
  } catch (e) {
    return NextResponse.json({ error: e instanceof Error ? e.message : String(e) }, { status: 400 });
  }
}
