"use client";

import { useCallback, useEffect, useState } from "react";
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
  const [msg, setMsg] = useState<string | null>(null);
  const [nom, setNom] = useState("");
  const [prenom, setPrenom] = useState("");
  const [secteur, setSecteur] = useState<Secteur>("college");

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

  return (
    <section className="rounded-xl border border-violet-200 bg-violet-50/40 p-4 space-y-3">
      <h3 className="text-sm font-bold text-slate-800">Enseignants (dossiers OCR)</h3>
      <p className="text-xs text-slate-600 leading-relaxed">
        Liste utilisée pour ranger automatiquement les PDF enseignants. Indépendante du personnel OGEC
        et du catalogue de répartition des classes.
      </p>
      {loading ? (
        <p className="text-xs text-slate-500">Chargement…</p>
      ) : (
        <p className="text-sm font-medium text-slate-800">{enseignants.length} enseignant(s).</p>
      )}
      <div className="flex flex-wrap gap-2 items-end">
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
