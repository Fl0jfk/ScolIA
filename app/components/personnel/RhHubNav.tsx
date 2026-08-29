"use client";

import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";

export type RhHubTab = "dashboard" | "pilotage";

const TABS: { id: RhHubTab; label: string; pilotageOnly?: boolean }[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "pilotage", label: "Pilotage RH", pilotageOnly: true },
];

export default function RhHubNav({
  active,
  onChange,
  canPilotage,
}: {
  active: RhHubTab;
  onChange: (tab: RhHubTab) => void;
  canPilotage: boolean;
}) {
  const tabs = TABS.map((t) => ({
    id: t.id,
    label: t.label,
    hidden: Boolean(t.pilotageOnly && !canPilotage),
  }));

  return <ModuleTabNav tabs={tabs} active={active} onChange={onChange} className="mb-6" />;
}

export type RhPilotageSection =
  | "overview"
  | "validations"
  | "annuaire"
  | "admin"
  | "onboarding"
  | "registre";

export function RhPilotageNav({
  active,
  onChange,
}: {
  active: RhPilotageSection;
  onChange: (section: RhPilotageSection) => void;
}) {
  const tabs: { id: RhPilotageSection; label: string }[] = [
    { id: "overview", label: "Vue d'ensemble" },
    { id: "validations", label: "Dossiers à valider" },
    { id: "annuaire", label: "Annuaire" },
    { id: "admin", label: "Entrées / sorties" },
    { id: "onboarding", label: "Nouveaux arrivants" },
    { id: "registre", label: "Registre" },
  ];

  return (
    <ModuleTabNav
      tabs={tabs}
      active={active}
      onChange={onChange}
      className="mb-4"
    />
  );
}
