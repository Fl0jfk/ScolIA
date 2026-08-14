"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { SettingsSection, settingsInputClass } from "@/app/components/settings/SettingsChrome";
import type { SettingsTravelsConfig } from "@/app/lib/settings-page-model";
import { dash } from "@/app/lib/dashboard-brand";

export default function SettingsTravelsPanel({
  travelsCfg,
  setTravelsCfg,
  saving,
  saveSection,
}: {
  travelsCfg: SettingsTravelsConfig;
  setTravelsCfg: Dispatch<SetStateAction<SettingsTravelsConfig>>;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  return (
    <SettingsSection icon="🚌" title="Sorties scolaires" description="Transporteurs et pied de page des PDF.">
      <label className="block text-sm font-bold">Texte pied de page PDF</label>
      <input
        className={settingsInputClass}
        value={travelsCfg.pdfFooterText || ""}
        onChange={(e) => setTravelsCfg({ ...travelsCfg, pdfFooterText: e.target.value })}
      />
      <p className="text-sm font-bold">Transporteurs</p>
      {travelsCfg.transportProviders.map((p, idx) => (
        <div key={idx} className="grid grid-cols-2 gap-2">
          <input
            className={settingsInputClass}
            placeholder="Nom"
            value={p.name}
            onChange={(e) => {
              const copy = [...travelsCfg.transportProviders];
              copy[idx] = { ...copy[idx], name: e.target.value };
              setTravelsCfg({ ...travelsCfg, transportProviders: copy });
            }}
          />
          <input
            className={settingsInputClass}
            placeholder="E-mail"
            type="email"
            value={p.email}
            onChange={(e) => {
              const copy = [...travelsCfg.transportProviders];
              copy[idx] = { ...copy[idx], email: e.target.value };
              setTravelsCfg({ ...travelsCfg, transportProviders: copy });
            }}
          />
        </div>
      ))}
      <button
        type="button"
        className={`text-sm font-semibold ${dash.textPrimary}`}
        onClick={() =>
          setTravelsCfg({
            ...travelsCfg,
            transportProviders: [...travelsCfg.transportProviders, { name: "", email: "" }],
          })
        }
      >
        + Transporteur
      </button>
      <ModuleButton
        variant="primary"
        disabled={saving}
        onClick={() => saveSection("travels", travelsCfg)}
      >
        Enregistrer sorties scolaires
      </ModuleButton>
    </SettingsSection>
  );
}
