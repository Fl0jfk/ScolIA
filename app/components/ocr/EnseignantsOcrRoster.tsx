"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { settingsInputClass } from "@/app/components/settings/SettingsChrome";
import type { EnseignantConfig } from "@/app/lib/enseignants-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

const SECTEURS: Array<{ key: Secteur; label: string }> = [
  { key: "ecole", label: "École" },
  { key: "college", label: "Collège" },
  { key: "lycee", label: "Lycée" },
];

export default function EnseignantsOcrRoster() {
  const [enseignants, setEnseignants] = useState<EnseignantConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importMode, setImportMode] = useState<"merge" | "replace">("merge");
  const [msg, setMsg] = useState<string | null>(null);
  const [nom, setNom] = useState("");
  const [prenom, setPrenom] = useState("");
  const [secteur, setSecteur] = useState<Secteur>("college");
  const fileInputRef = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/enseignants", { cache: "no-store" });
      const j = await res.json();
      if (res.ok) setEnseignants(Array.isArray(j.enseignants) ? j.enseignants : []);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function save(next: EnseignantConfig[]) {
    setSaving(true);
    setMsg(null);
    try {
      const res = await fetch("/api/enseignants", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enseignants: next }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setEnseignants(j.enseignants || next);
      setMsg(`${(j.enseignants || next).length} enseignant(s) enregistré(s).`);
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  }

  async function importExcel(file: File) {
    setImporting(true);
    setMsg(null);
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("mode", importMode);
      const res = await fetch("/api/enseignants/import", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Import impossible");
      await load();
      const extra =
        Array.isArray(j.warnings) && j.warnings.length
          ? ` · ${j.warnings.length} alerte(s) (prof multi-cycles).`
          : "";
      setMsg((j.message || "Import terminé.") + extra);
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Erreur import");
    } finally {
      setImporting(false);
    }
  }

  return (
    <section className="rounded-xl border border-violet-200 bg-violet-50/40 p-4 space-y-3">
      <h3 className="text-sm font-bold text-slate-800">Enseignants (dossiers OCR)</h3>
      <p className="text-xs text-slate-600 leading-relaxed">
        Liste pour le matching OCR. Le rangement OneDrive utilise un chemin commun « Dossier
        enseignants » (école / collège / lycée fusionnés). Le cycle sert à filtrer qui peut matcher
        selon les flux de la personne connectée.
      </p>
      <div className="rounded-lg border border-violet-100 bg-white/80 px-3 py-2 text-xs text-slate-600 space-y-2">
        <p className="font-semibold text-slate-700">Modèle Excel — 1re ligne = en-têtes exacts</p>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-violet-100">
                <th className="py-1 pr-2 font-semibold">Nom</th>
                <th className="py-1 pr-2 font-semibold">Prénom</th>
                <th className="py-1 pr-2 font-semibold">Liste des classes</th>
                <th className="py-1 pr-2 font-semibold">Email personnel</th>
                <th className="py-1 pr-2 font-semibold">Email professionnel</th>
              </tr>
            </thead>
            <tbody className="text-slate-500">
              <tr>
                <td className="py-1 pr-2">HEBERT</td>
                <td className="py-1 pr-2">Pascal</td>
                <td className="py-1 pr-2">1C, TB, 2E</td>
                <td className="py-1 pr-2">perso@…</td>
                <td className="py-1 pr-2">pascal.hebert@…</td>
              </tr>
            </tbody>
          </table>
        </div>
        <ul className="list-disc pl-4 space-y-0.5">
          <li>
            <strong>Obligatoire :</strong> Nom, Prénom
          </li>
          <li>
            <strong>Cycle :</strong> « Liste des classes » (Charlemagne) <em>ou</em> colonne « Cycle »
          </li>
          <li>
            <strong>Emails :</strong> les deux si vous les avez — l&apos;OCR cherche l&apos;un ou l&apos;autre sur le PDF
          </li>
        </ul>
      </div>
      <div className="flex flex-wrap gap-2 items-end">
        <label className="text-xs font-semibold text-slate-600">
          Mode import
          <select
            className={`${settingsInputClass} mt-1`}
            value={importMode}
            onChange={(e) => setImportMode(e.target.value as "merge" | "replace")}
          >
            <option value="merge">Fusionner avec la liste actuelle</option>
            <option value="replace">Remplacer toute la liste</option>
          </select>
        </label>
        <ModuleButton
          variant="secondary"
          disabled={importing || saving}
          onClick={() => fileInputRef.current?.click()}
          className="px-3 py-2 text-sm"
        >
          {importing ? "Import…" : "Importer Excel profs"}
        </ModuleButton>
        <input
          ref={fileInputRef}
          type="file"
          accept=".xlsx,.xls"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void importExcel(f);
            e.target.value = "";
          }}
        />
      </div>
      {loading ? (
        <p className="text-xs text-slate-500">Chargement…</p>
      ) : (
        <p className="text-sm font-medium text-slate-800">{enseignants.length} enseignant(s).</p>
      )}
      <div className="flex flex-wrap gap-2 items-end border-t border-violet-100 pt-3">
        <label className="text-xs font-semibold text-slate-600">
          Nom
          <input className={`${settingsInputClass} mt-1`} value={nom} onChange={(e) => setNom(e.target.value)} />
        </label>
        <label className="text-xs font-semibold text-slate-600">
          Prénom
          <input
            className={`${settingsInputClass} mt-1`}
            value={prenom}
            onChange={(e) => setPrenom(e.target.value)}
          />
        </label>
        <label className="text-xs font-semibold text-slate-600">
          Cycle
          <select
            className={`${settingsInputClass} mt-1`}
            value={secteur}
            onChange={(e) => setSecteur(e.target.value as Secteur)}
          >
            {SECTEURS.map((s) => (
              <option key={s.key} value={s.key}>
                {s.label}
              </option>
            ))}
          </select>
        </label>
        <ModuleButton
          variant="secondary"
          disabled={saving || !nom.trim() || !prenom.trim()}
          onClick={() => {
            void save([
              ...enseignants,
              { id: "", nom: nom.trim(), prenom: prenom.trim(), folderName: "", secteur },
            ]);
            setNom("");
            setPrenom("");
          }}
          className="px-3 py-2 text-sm"
        >
          Ajouter
        </ModuleButton>
      </div>
      {enseignants.length > 0 ? (
        <ul className="max-h-48 overflow-y-auto rounded-lg border border-violet-100 bg-white text-sm divide-y">
          {enseignants.map((e) => (
            <li key={e.id} className="flex items-center justify-between gap-2 px-3 py-1.5">
              <span>
                {e.nom} {e.prenom}{" "}
                <span className="text-xs text-slate-500">
                  · {e.secteur} · {e.folderName}
                  {e.email || e.emailPro
                    ? ` · ${[e.email, e.emailPro].filter(Boolean).join(" / ")}`
                    : ""}
                </span>
              </span>
              <button
                type="button"
                className="text-xs font-semibold text-rose-600"
                onClick={() => void save(enseignants.filter((x) => x.id !== e.id))}
              >
                Retirer
              </button>
            </li>
          ))}
        </ul>
      ) : null}
      {msg ? <p className="text-xs text-slate-600">{msg}</p> : null}
    </section>
  );
}
