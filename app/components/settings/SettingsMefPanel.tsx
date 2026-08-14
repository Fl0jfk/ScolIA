"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";

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
}) {
  return (
    <div className="bg-white rounded-2xl border p-6 space-y-5">
      <p className="text-sm text-slate-600">
        Table des formations (libellés ou codes MEF) pour l&apos;agent IA OneDrive : chaque{" "}
        <strong>organisation</strong> a son propre fichier sur S3 (
        <code className="text-xs bg-slate-100 px-1 rounded">settings/mef-secteurs.json</code>
        ). Une ligne = une formation. Les élèves sont rattachés via le champ <code className="text-xs">mef</code>{" "}
        de la liste élèves.
      </p>
      <div>
        <label className="block text-sm font-bold text-slate-700 mb-1">Lycée</label>
        <textarea
          className="w-full border rounded-xl p-3 text-sm font-mono min-h-[140px]"
          placeholder={"2NDE GENERALE ET TECHNOLOGIQUE\nTERMINALE GENERALE\n…"}
          value={mefLycee}
          onChange={(e) => setMefLycee(e.target.value)}
        />
      </div>
      <div>
        <label className="block text-sm font-bold text-slate-700 mb-1">Collège</label>
        <textarea
          className="w-full border rounded-xl p-3 text-sm font-mono min-h-[120px]"
          placeholder={"6EME\n5EME\n3EME\n…"}
          value={mefCollege}
          onChange={(e) => setMefCollege(e.target.value)}
        />
      </div>
      <div>
        <label className="block text-sm font-bold text-slate-700 mb-1">École</label>
        <textarea
          className="w-full border rounded-xl p-3 text-sm font-mono min-h-[160px]"
          placeholder={"Cycle 2 - COURS PREPARATOIRE\nCycle 1 - GRANDE SECTION\n…"}
          value={mefEcole}
          onChange={(e) => setMefEcole(e.target.value)}
        />
      </div>
      {mefMessage && (
        <p className={`text-sm ${mefMessage.startsWith("Erreur") ? "text-red-600" : "text-green-700"}`}>
          {mefMessage}
        </p>
      )}
      <div className="flex flex-wrap gap-3">
        <ModuleButton variant="primary" disabled={saving} onClick={onSave}>
          Enregistrer les formations MEF
        </ModuleButton>
        <label className="cursor-pointer text-sm font-bold text-indigo-600 hover:underline self-center">
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
    </div>
  );
}
