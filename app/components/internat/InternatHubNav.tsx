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

const TABS: { id: InternatTab; label: string }[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "installation", label: "Installation / RDV" },
  { id: "chambres", label: "Chambres" },
  { id: "internes", label: "Internes" },
  { id: "sorties", label: "Sorties" },
  { id: "appel", label: "Appel" },
  { id: "historique", label: "Historique" },
  { id: "etudes", label: "Études" },
  { id: "surveillants", label: "Surveillants" },
  { id: "suivi", label: "Suivi éducatif" },
  { id: "communication", label: "Communication" },
  { id: "activites", label: "Événements" },
  { id: "alertes", label: "Alertes" },
];

export default function InternatHubNav({
  active,
  onChange,
}: {
  active: InternatTab;
  onChange: (tab: InternatTab) => void;
}) {
  return <ModuleTabNav tabs={TABS} active={active} onChange={onChange} className="mb-8" />;
}
