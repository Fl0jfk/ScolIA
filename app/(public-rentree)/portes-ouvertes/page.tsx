import type { Metadata } from "next";
import { loadAppConfig } from "@/app/lib/app-config";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import { notFound } from "next/navigation";
import {
  buildPortesOuvertesToolPayload,
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
} from "@/app/lib/portes-ouvertes-db";
import {
  cyclesFromActiveEstablishments,
  PORTES_OUVERTES_CYCLE_LABELS,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import PortesOuvertesClient from "./PortesOuvertesClient";

export async function generateMetadata(): Promise<Metadata> {
  const toolbox = await getToolboxConfig();
  const title = toolbox.tools["portes-ouvertes"].title?.trim() || "Portes ouvertes";
  return {
    title,
    description: "Inscription aux portes ouvertes de l’établissement.",
  };
}

export default async function PortesOuvertesPage() {
  const toolbox = await getToolboxConfig();
  if (!toolbox.tools["portes-ouvertes"].enabled) notFound();

  const [payload, registrations, bundle] = await Promise.all([
    buildPortesOuvertesToolPayload(),
    listPortesOuvertesRegistrations(),
    loadAppConfig(),
  ]);
  const counts = countRegistrationsBySlot(registrations);
  const cycles = cyclesFromActiveEstablishments(bundle.establishments);
  const cycleLabels: Record<PortesOuvertesCycle, string> = { ...PORTES_OUVERTES_CYCLE_LABELS };
  for (const e of bundle.establishments) {
    if (e.kind === "ecole" || e.kind === "college" || e.kind === "lycee") {
      cycleLabels[e.kind] = e.label || cycleLabels[e.kind];
    }
  }

  const slots = payload.slots.map((s) => ({
    ...s,
    registeredCount: counts[s.id] || 0,
    remaining:
      typeof s.maxPlaces === "number"
        ? Math.max(0, s.maxPlaces - (counts[s.id] || 0))
        : null,
  }));

  return (
    <PortesOuvertesClient
      po={{
        enabled: true,
        title: payload.title,
        intro: payload.intro,
        address: payload.address,
        mapsUrl: payload.mapsUrl,
        notifyEmail: payload.notifyEmail,
        consentLabel: payload.consentLabel,
        slots,
      }}
      cycles={cycles}
      cycleLabels={cycleLabels}
    />
  );
}
