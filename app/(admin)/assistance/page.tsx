"use client";

import { useUser } from "@clerk/nextjs";
import { useEffect, useRef, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

const MAX_FILES = 8;

type ScopeOption = { id: string; label: string };

export default function AssistancePage() {
  const { user, isLoaded } = useUser();
  const fileRef = useRef<HTMLInputElement>(null);
  const [options, setOptions] = useState<ScopeOption[]>([]);
  const [scope, setScope] = useState("");
  const [description, setDescription] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (!isLoaded || !user) return;
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const res = await fetch("/api/assistance", { cache: "no-store" });
        const data = await res.json();
        if (!cancelled && res.ok && Array.isArray(data.options)) {
          setOptions(data.options);
          setScope((prev) => prev || data.options[0]?.id || "");
        }
      } catch {
        if (!cancelled) setError("Impossible de charger les options.");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [isLoaded, user]);

  const onPickFiles = (list: FileList | null) => {
    if (!list) return;
    setFiles((prev) => [...prev, ...Array.from(list)].slice(0, MAX_FILES));
  };

  const removeFile = (index: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const canSubmit = scope && description.trim().length >= 15 && !submitting;

  const submit = async () => {
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    setSuccess(null);
    try {
      const fd = new FormData();
      fd.append("scope", scope);
      fd.append("description", description.trim());
      for (const f of files) fd.append("files", f);

      const res = await fetch("/api/assistance", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Envoi impossible.");

      setSuccess(data.message || "Demande envoyée.");
      setDescription("");
      setFiles([]);
      if (fileRef.current) fileRef.current.value = "";
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur inattendue.");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-3xl" className="pb-16">
      <ModulePageHeader
        title="Assistance"
        description="Signalez un problème sur le tableau de bord ou un module de l'intranet. Votre message sera transmis à l'équipe technique avec les pièces jointes éventuelles."
      />

      <div className="rounded-3xl border border-slate-200 bg-white p-6 md:p-8 shadow-sm space-y-6">
        {loading ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : (
          <>
            <div>
              <label htmlFor="assistance-scope" className="block text-sm font-bold text-slate-700 mb-1.5">
                Où avez-vous rencontré le problème ?
              </label>
              <select
                id="assistance-scope"
                className="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                value={scope}
                onChange={(e) => setScope(e.target.value)}
              >
                {options.map((o) => (
                  <option key={o.id} value={o.id}>
                    {o.label}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label htmlFor="assistance-desc" className="block text-sm font-bold text-slate-700 mb-1.5">
                Description du problème
              </label>
              <textarea
                id="assistance-desc"
                rows={7}
                className="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm resize-y min-h-[140px]"
                placeholder="Décrivez ce qui ne fonctionne pas, les étapes pour reproduire le bug, le message d'erreur éventuel…"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
              <p className="mt-1 text-xs text-slate-400">Minimum 15 caractères.</p>
            </div>

            <div>
              <label className="block text-sm font-bold text-slate-700 mb-1.5">
                Pièces jointes (optionnel)
              </label>
              <p className="text-xs text-slate-500 mb-2">
                PDF, photos (JPG, PNG…) — {MAX_FILES} fichiers max, 12 Mo chacun.
              </p>
              <input
                ref={fileRef}
                type="file"
                multiple
                accept="image/*,application/pdf,.pdf,.doc,.docx,.xls,.xlsx"
                className="block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:font-semibold file:text-indigo-700"
                onChange={(e) => onPickFiles(e.target.files)}
              />
              {files.length > 0 && (
                <ul className="mt-3 space-y-2">
                  {files.map((f, i) => (
                    <li
                      key={`${f.name}-${i}`}
                      className="flex items-center justify-between gap-2 rounded-lg bg-slate-50 px-3 py-2 text-sm"
                    >
                      <span className="truncate text-slate-700">{f.name}</span>
                      <button
                        type="button"
                        onClick={() => removeFile(i)}
                        className="shrink-0 text-xs font-bold text-red-600 hover:underline"
                      >
                        Retirer
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            {error && (
              <p className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
                {error}
              </p>
            )}

            {success && (
              <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
                {success}
              </p>
            )}

            <button
              type="button"
              disabled={!canSubmit}
              onClick={() => void submit()}
              className="w-full sm:w-auto rounded-xl bg-indigo-600 px-6 py-3 text-sm font-bold text-white hover:bg-indigo-700 disabled:opacity-50 transition"
            >
              {submitting ? "Envoi en cours…" : "Envoyer la demande"}
            </button>
          </>
        )}
      </div>
    </ModulePageShell>
  );
}
