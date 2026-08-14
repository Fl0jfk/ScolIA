"use client";

export type StaffDossierTabId =
  | "identite"
  | "docs"
  | "formations"
  | "habilitations"
  | "medecine"
  | "entretiens"
  | "onboarding"
  | "offboarding";

export function staffDossierTabs(opts: { hasOnboarding: boolean; showOffboarding: boolean }) {
  return [
    { id: "identite" as const, label: "Identité & contrat" },
    { id: "docs" as const, label: "Documents" },
    { id: "formations" as const, label: "Formations" },
    { id: "habilitations" as const, label: "Habilitations" },
    { id: "medecine" as const, label: "Médecine du travail" },
    { id: "entretiens" as const, label: "Entretiens" },
    ...(opts.hasOnboarding ? [{ id: "onboarding" as const, label: "Onboarding" }] : []),
    ...(opts.showOffboarding ? [{ id: "offboarding" as const, label: "Offboarding" }] : []),
  ];
}

export function StaffDossierTabBar({
  tabs,
  tab,
  onChange,
}: {
  tabs: Array<{ id: StaffDossierTabId; label: string }>;
  tab: StaffDossierTabId;
  onChange: (id: StaffDossierTabId) => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {tabs.map((t) => (
        <button
          key={t.id}
          type="button"
          onClick={() => onChange(t.id)}
          className={`px-4 py-2 rounded-xl text-xs font-bold transition ${
            tab === t.id ? "bg-indigo-600 text-white" : "bg-slate-100 text-slate-700 hover:bg-slate-200"
          }`}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
