import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  buildPortesOuvertesToolPayload,
  listPortesOuvertesRegistrations,
  listPortesOuvertesStaff,
} from "@/app/lib/portes-ouvertes-db";
import {
  cyclesFromActiveEstablishments,
  PORTES_OUVERTES_CYCLE_LABELS,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import { renderPortesOuvertesPlanningPdf } from "@/app/lib/portes-ouvertes-planning-pdf";

export async function GET() {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

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

  const pdf = renderPortesOuvertesPlanningPdf({
    title: payload.title,
    address: payload.address,
    cycleLabels,
    slots: payload.slots.filter((s) => !s.cycle || cycles.includes(s.cycle)),
    registrations,
    staff,
  });

  return new NextResponse(new Uint8Array(pdf), {
    status: 200,
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="portes-ouvertes-planning.pdf"`,
      "Cache-Control": "no-store",
    },
  });
}
