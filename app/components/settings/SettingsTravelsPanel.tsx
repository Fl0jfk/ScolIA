"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import type { SettingsTravelsConfig } from "@/app/lib/settings-page-model";

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
    <div className="bg-white rounded-2xl border p-6 space-y-4">
      <p className="text-sm text-slate-600">Transporteurs et pied de page des PDF de sorties scolaires.</p>
      <label className="block text-sm font-bold">Texte pied de page PDF</label>
      <input
        className="w-full border rounded-xl p-3"
        value={travelsCfg.pdfFooterText || ""}
        onChange={(e) => setTravelsCfg({ ...travelsCfg, pdfFooterText: e.target.value })}
      />
      <p className="text-sm font-bold">Transporteurs</p>
      {travelsCfg.transportProviders.map((p, idx) => (
        <div key={idx} className="grid grid-cols-2 gap-2">
          <input
            className="border rounded-lg p-2 text-sm"
            placeholder="Nom"
            value={p.name}
            onChange={(e) => {
              const copy = [...travelsCfg.transportProviders];
              copy[idx] = { ...copy[idx], name: e.target.value };
              setTravelsCfg({ ...travelsCfg, transportProviders: copy });
            }}
          />
          <input
            className="border rounded-lg p-2 text-sm"
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
        className="text-indigo-600 text-sm font-bold"
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
    </div>
  );
}
