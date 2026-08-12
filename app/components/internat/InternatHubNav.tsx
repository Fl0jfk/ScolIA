"use client";

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
  { id: "installation", label: "Installation / RDV" },
];

export default function InternatHubNav({
  active,
  onChange,
}: {
  active: InternatTab;
  onChange: (tab: InternatTab) => void;
}) {
  return (
    <nav className="flex flex-wrap gap-2 mb-8">
      {TABS.map((tab) => (
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
