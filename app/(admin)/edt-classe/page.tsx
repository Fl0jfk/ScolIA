"use client";

import { useSearchParams } from "next/navigation";
import ClassPlanningPanel from "@/app/components/personnel/ClassPlanningPanel";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function EdtClassePage() {
  const searchParams = useSearchParams();
  const initialClasse = searchParams.get("classe")?.trim() || "";

  return (
    <ModulePageShell maxWidthClass="max-w-[1400px]">
      <ModulePageHeader
        eyebrow="RH"
        title="EDT par classe"
        description="Vue agrégée des emplois du temps profs pour une classe — semaines types A/B et remplacements."
      />
      <ClassPlanningPanel initialClasse={initialClasse} />
    </ModulePageShell>
  );
}
