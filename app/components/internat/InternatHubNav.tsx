"use client";

import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";

export type InternatTab =
  | "dashboard"
  | "chambres"
  | "internes"
  | "sorties"
  | "appel"
  | "historique"
  | "etudes"
  | "surveillants"
  | "suivi"
  | "communication"
  | "activites"
  | "alertes"
  | "installation";

/** Onglets visibles en présentation — le reste reste accessible via ?tab= */
const VISIBLE_TABS: { id: InternatTab; label: string }[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "internes", label: "Internes" },
  { id: "appel", label: "Appel" },
  { id: "chambres", label: "Chambres" },
  { id: "sorties", label: "Sorties" },
];

export default function InternatHubNav({
  active,
  onChange,
}: {
  active: InternatTab;
  onChange: (tab: InternatTab) => void;
}) {
  const tabs =
    VISIBLE_TABS.some((t) => t.id === active) || active === "installation"
      ? VISIBLE_TABS
      : [...VISIBLE_TABS, { id: active, label: active }];

  return <ModuleTabNav tabs={tabs} active={active} onChange={onChange} className="mb-6" />;
}
