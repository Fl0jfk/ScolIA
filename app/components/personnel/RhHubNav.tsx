"use client";

export type RhHubTab =
  | "dashboard"
  | "annuaire"
  | "admin"
  | "onboarding"
  | "registre"
  | "absences"
  | "hse"
  | "demande"
  | "organigramme"
  | "deposit";

const TABS: { id: RhHubTab; label: string; manageOnly?: boolean; hseOnly?: boolean }[] = [
  { id: "dashboard", label: "Tableau de bord" },
  { id: "absences", label: "Absences" },
  { id: "hse", label: "Demandes HSE", hseOnly: true },
  { id: "demande", label: "Demande RH" },
  { id: "annuaire", label: "Annuaire" },
  { id: "admin", label: "Entrées / sorties", manageOnly: true },
  { id: "onboarding", label: "Nouveaux arrivants", manageOnly: true },
  { id: "registre", label: "Registre", manageOnly: true },
  { id: "organigramme", label: "Organigramme" },
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
  const visible = TABS.filter((t) => {
    if (t.hseOnly) return canAccessHse;
    if (t.manageOnly) return canManage;
    return true;
  });

  return (
    <nav className="flex flex-wrap gap-2 mb-6">
      {visible.map((tab) => (
        <button
          key={tab.id}
          type="button"
          onClick={() => onChange(tab.id)}
          className={`px-4 py-2 rounded-xl text-sm font-bold transition-colors ${
            active === tab.id
              ? "bg-indigo-600 text-white shadow-md"
              : "bg-white border border-slate-200 text-slate-600 hover:border-indigo-300"
          }`}
        >
          {tab.label}
        </button>
      ))}
    </nav>
  );
}
