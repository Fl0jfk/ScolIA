"use client";

import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";

export type RhHubTab =
  | "dashboard"
  | "annuaire"
  | "admin"
  | "onboarding"
  | "registre"
  | "absences"
  | "hse"
  | "demande"
  | "planning";

const TABS: {
  id: RhHubTab;
  label: string;
  directoryOnly?: boolean;
  manageOnly?: boolean;
  hseOnly?: boolean;
  ogecOnly?: boolean;
}[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "absences", label: "Absences" },
  { id: "hse", label: "Demandes HSE", hseOnly: true },
  { id: "demande", label: "Demande RH", ogecOnly: true },
  { id: "planning", label: "Planning" },
  { id: "annuaire", label: "Annuaire", directoryOnly: true },
  { id: "admin", label: "Entrées / sorties", directoryOnly: true },
  { id: "onboarding", label: "Nouveaux arrivants", directoryOnly: true },
  { id: "registre", label: "Registre", directoryOnly: true },
];

export default function RhHubNav({
  active,
  onChange,
  canDirectory,
  canAccessHse,
  canAccessDemandeRh,
}: {
  active: RhHubTab;
  onChange: (tab: RhHubTab) => void;
  canDirectory: boolean;
  canAccessHse: boolean;
  canAccessDemandeRh: boolean;
}) {
  const tabs = TABS.map((t) => ({
    id: t.id,
    label: t.label,
    hidden: Boolean(
      (t.hseOnly && !canAccessHse) ||
        (t.ogecOnly && !canAccessDemandeRh) ||
        ((t.directoryOnly || t.manageOnly) && !canDirectory),
    ),
  }));

  return <ModuleTabNav tabs={tabs} active={active} onChange={onChange} className="mb-6" />;
}
