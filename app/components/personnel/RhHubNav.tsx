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
  | "planning"
  | "organigramme"
  | "deposit";

const TABS: { id: RhHubTab; label: string; manageOnly?: boolean; hseOnly?: boolean }[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "absences", label: "Absences" },
  { id: "hse", label: "Demandes HSE", hseOnly: true },
  { id: "demande", label: "Demande RH" },
  { id: "planning", label: "Planning" },
  { id: "annuaire", label: "Annuaire" },
  { id: "admin", label: "Entrées / sorties", manageOnly: true },
  { id: "onboarding", label: "Nouveaux arrivants", manageOnly: true },
  { id: "registre", label: "Registre", manageOnly: true },
  { id: "organigramme", label: "Organisation" },
  { id: "deposit", label: "Dépôt IA", manageOnly: true },
];

export default function RhHubNav({
  active,
  onChange,
  canManage,
  canAccessHse,
}: {
  active: RhHubTab;
  onChange: (tab: RhHubTab) => void;
  canManage: boolean;
  canAccessHse: boolean;
}) {
  const tabs = TABS.map((t) => ({
    id: t.id,
    label: t.label,
    hidden: Boolean((t.hseOnly && !canAccessHse) || (t.manageOnly && !canManage)),
  }));

  return <ModuleTabNav tabs={tabs} active={active} onChange={onChange} className="mb-6" />;
}
