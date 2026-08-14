"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { SettingsNotice, SettingsSection, settingsInputClass } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

export default function SettingsMefPanel({
  mefLycee,
  setMefLycee,
  mefCollege,
  setMefCollege,
  mefEcole,
  setMefEcole,
  mefMessage,
  saving,
  onSave,
  onImportJson,
  activeEstablishmentKinds,
}: {
  mefLycee: string;
  setMefLycee: Dispatch<SetStateAction<string>>;
  mefCollege: string;
  setMefCollege: Dispatch<SetStateAction<string>>;
  mefEcole: string;
  setMefEcole: Dispatch<SetStateAction<string>>;
  mefMessage: string | null;
  saving: boolean;
  onSave: () => void;
  onImportJson: (file: File) => void;
  activeEstablishmentKinds?: Set<string>;
}) {
  const showEcole = !activeEstablishmentKinds || activeEstablishmentKinds.has("ecole");
  const showCollege = !activeEstablishmentKinds || activeEstablishmentKinds.has("college");
  const showLycee = !activeEstablishmentKinds || activeEstablishmentKinds.has("lycee");
  const showAny = showEcole || showCollege || showLycee;

  return (
    <SettingsSection
      icon="📚"
      title="Formations MEF"
      description={
        <>
          Table des formations pour l&apos;agent IA OneDrive. Une ligne = une formation. Fichier S3{" "}
          <code className="rounded bg-white/70 px-1 text-xs">settings/mef-secteurs.json</code>.
        </>
      }
    >
      {!showAny && (
        <SettingsNotice tone="warn">
          Configurez d&apos;abord un établissement de type école, collège ou lycée dans l&apos;onglet Sites /
          directions pour saisir les formations MEF correspondantes.
        </SettingsNotice>
      )}
      {showLycee && (
        <div>
          <label className="block text-sm font-bold text-slate-700 mb-1">Lycée</label>
          <textarea
            className={`${settingsInputClass} min-h-[140px] font-mono`}
            placeholder={"2NDE GENERALE ET TECHNOLOGIQUE\nTERMINALE GENERALE\n…"}
            value={mefLycee}
            onChange={(e) => setMefLycee(e.target.value)}
          />
        </div>
      )}
      {showCollege && (
        <div>
          <label className="block text-sm font-bold text-slate-700 mb-1">Collège</label>
          <textarea
            className={`${settingsInputClass} min-h-[120px] font-mono`}
            placeholder={"6EME\n5EME\n3EME\n…"}
            value={mefCollege}
            onChange={(e) => setMefCollege(e.target.value)}
          />
        </div>
      )}
      {showEcole && (
        <div>
          <label className="block text-sm font-bold text-slate-700 mb-1">École</label>
          <textarea
            className={`${settingsInputClass} min-h-[160px] font-mono`}
            placeholder={"Cycle 2 - COURS PREPARATOIRE\nCycle 1 - GRANDE SECTION\n…"}
            value={mefEcole}
            onChange={(e) => setMefEcole(e.target.value)}
          />
        </div>
      )}
      {mefMessage && (
        <p className={`text-sm ${mefMessage.startsWith("Erreur") ? "text-red-600" : "text-green-700"}`}>
          {mefMessage}
        </p>
      )}
      <div className="flex flex-wrap gap-3">
        <ModuleButton variant="primary" disabled={saving || !showAny} onClick={onSave}>
          Enregistrer les formations MEF
        </ModuleButton>
        <label className={`cursor-pointer self-center text-sm font-semibold ${dash.textPrimary} hover:underline`}>
          Importer un .json
          <input
            type="file"
            accept=".json,application/json"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) onImportJson(f);
              e.target.value = "";
            }}
          />
        </label>
      </div>
    </SettingsSection>
  );
}
