"use client";

import EstablishmentPlanningPanel from "@/app/components/personnel/EstablishmentPlanningPanel";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function EdtEtablissementPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-[1200px]">
      <ModulePageHeader
        eyebrow="RH"
        title="EDT établissement"
        description="Vue direction — conflits salle/classe, couverture EDT et accès rapide aux emplois du temps par classe."
      />
      <EstablishmentPlanningPanel />
    </ModulePageShell>
  );
}
