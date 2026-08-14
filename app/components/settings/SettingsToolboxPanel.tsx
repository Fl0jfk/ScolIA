"use client";

import { SettingsSection, settingsPillClass } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

export default function SettingsToolboxPanel() {
  return (
    <SettingsSection
      icon="🧰"
      title="Boîte à outils saisonnière"
      description={
        <>
          QR code et répartition des classes. Rentrée + fournitures →{" "}
          <a href="/etablissement/evenements" className={`font-semibold underline ${dash.ink}`}>
            Événements
          </a>
          . Simulateur de tarifs →{" "}
          <a href="/etablissement/communication" className={`font-semibold underline ${dash.ink}`}>
            Communication
          </a>
          .
        </>
      }
    >
      <div className="flex flex-wrap gap-2">
        <a
          href="/toolbox"
          className="inline-flex rounded-2xl bg-[var(--dash-primary)] px-5 py-2.5 text-sm font-semibold text-white shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)] hover:opacity-95"
        >
          Ouvrir la boîte à outils →
        </a>
        <a href="/etablissement/evenements" className={settingsPillClass}>
          Événements →
        </a>
        <a href="/etablissement/communication" className={settingsPillClass}>
          Communication →
        </a>
      </div>
    </SettingsSection>
  );
}
