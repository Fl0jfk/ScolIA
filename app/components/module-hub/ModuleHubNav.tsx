"use client";

export type ModuleHubTab = {
  id: string;
  label: string;
  badge?: string | number | null;
};

export default function ModuleHubNav({
  tabs,
  active,
  onChange,
}: {
  tabs: ModuleHubTab[];
  active: string;
  onChange: (id: string) => void;
}) {
  return (
    <nav className="mb-6 flex flex-wrap gap-2" aria-label="Sous-modules">
      {tabs.map((tab) => {
        const isActive = tab.id === active;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onChange(tab.id)}
            className={`inline-flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-bold transition-all ${
              isActive
                ? "bg-[var(--dash-primary)] text-white shadow-md shadow-[color:var(--dash-primary)]/25"
                : "border border-[color:var(--dash-border)] bg-white/80 text-[var(--dash-ink)] backdrop-blur hover:border-[color:var(--dash-primary)]/40"
            }`}
          >
            {tab.label}
            {tab.badge != null && tab.badge !== "" && Number(tab.badge) !== 0 ? (
              <span
                className={`rounded-md px-1.5 py-0.5 text-[10px] font-black ${
                  isActive ? "bg-white/25 text-white" : "bg-[var(--dash-primary)] text-white"
                }`}
              >
                {tab.badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </nav>
  );
}
