import { loadAppConfig } from "@/app/lib/app-config";
import {
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
} from "@/app/lib/portes-ouvertes-storage";
import { cyclesFromActiveEstablishments } from "@/app/lib/portes-ouvertes-types";
import { requirePortesOuvertesPublicPage } from "@/app/lib/toolbox-public-gate";
import PortesOuvertesClient from "./PortesOuvertesClient";

export default async function PortesOuvertesPage() {
  const po = await requirePortesOuvertesPublicPage();
  const [registrations, bundle] = await Promise.all([
    listPortesOuvertesRegistrations(),
    loadAppConfig(),
  ]);
  const counts = countRegistrationsBySlot(registrations);
  const cycles = cyclesFromActiveEstablishments(bundle.establishments);
  const slots = po.slots.map((s) => {
    const registeredCount = counts[s.id] || 0;
    return {
      ...s,
      registeredCount,
      remaining:
        typeof s.maxPlaces === "number" ? Math.max(0, s.maxPlaces - registeredCount) : null,
    };
  });

  return <PortesOuvertesClient po={{ ...po, slots }} cycles={cycles} />;
}
