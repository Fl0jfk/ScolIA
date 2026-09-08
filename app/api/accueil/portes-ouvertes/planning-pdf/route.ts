import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import { parisDateKey } from "@/app/lib/paris-time";
import {
  buildPortesOuvertesToolPayload,
  listPortesOuvertesRegistrations,
  listPortesOuvertesStaff,
} from "@/app/lib/portes-ouvertes-db";
import {
  cyclesFromActiveEstablishments,
  isPortesOuvertesCycle,
  PORTES_OUVERTES_CYCLE_LABELS,
  PORTES_OUVERTES_CYCLES,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import { renderPortesOuvertesPlanningPdf } from "@/app/lib/portes-ouvertes-planning-pdf";

export async function GET(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const dayParam = url.searchParams.get("day")?.trim() || "";
  const cycleParam = url.searchParams.get("cycle")?.trim() || "";
  const dayKey = /^\d{4}-\d{2}-\d{2}$/.test(dayParam) ? dayParam : "";
  const cycleFilter: PortesOuvertesCycle | "all" =
    cycleParam && isPortesOuvertesCycle(cycleParam) ? cycleParam : "all";

  const [payload, registrations, staff, bundle] = await Promise.all([
    buildPortesOuvertesToolPayload(),
    listPortesOuvertesRegistrations(),
    listPortesOuvertesStaff(),
    loadAppConfig(),
  ]);

  const cycles = cyclesFromActiveEstablishments(bundle.establishments);
  const cycleLabels: Partial<Record<PortesOuvertesCycle, string>> = {
    ...PORTES_OUVERTES_CYCLE_LABELS,
  };
  for (const e of bundle.establishments) {
    if (e.kind === "ecole" || e.kind === "college" || e.kind === "lycee") {
      cycleLabels[e.kind] = e.label || cycleLabels[e.kind];
    }
  }

  let slots = payload.slots.filter((s) => !s.cycle || cycles.includes(s.cycle));
  if (cycleFilter !== "all") {
    slots = slots.filter((s) => !s.cycle || s.cycle === cycleFilter);
  }
  if (dayKey) {
    slots = slots.filter((s) => parisDateKey(s.startAt) === dayKey);
  }

  const slotIds = new Set(slots.map((s) => s.id));
  const filteredRegs = registrations.filter((r) => slotIds.has(r.slotId));
  const filteredStaff = staff.filter((x) => slotIds.has(x.slotId));

  const cycleTitle =
    cycleFilter !== "all"
      ? cycleLabels[cycleFilter] || PORTES_OUVERTES_CYCLE_LABELS[cycleFilter]
      : null;
  const pdfTitle = [payload.title, cycleTitle].filter(Boolean).join(" — ");

  const pdf = renderPortesOuvertesPlanningPdf({
    title: pdfTitle,
    address: payload.address,
    cycleLabels,
    cycles:
      cycleFilter !== "all"
        ? [cycleFilter]
        : PORTES_OUVERTES_CYCLES.filter((c) => cycles.includes(c)),
    slots,
    registrations: filteredRegs,
    staff: filteredStaff,
  });

  const fileBits = [
    "portes-ouvertes-planning",
    dayKey || null,
    cycleFilter !== "all" ? cycleFilter : null,
  ].filter(Boolean);
  const filename = `${fileBits.join("-")}.pdf`;

  return new NextResponse(new Uint8Array(pdf), {
    status: 200,
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="${filename}"`,
      "Cache-Control": "no-store",
    },
  });
}
