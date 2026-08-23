"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { useEffect, useRef, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

const MAX_FILES = 8;

type ScopeOption = { id: string; label: string };

export default function AssistancePage() {
  const { user, isLoaded } = useSessionUser();
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

      <ModuleCard bodyClassName="space-y-6 p-6 md:p-8">
        {loading ? (
          <p className={`text-sm ${dash.textMid}`}>Chargement…</p>
        ) : (
          <>
            <div>
              <label htmlFor="assistance-scope" className={`mb-1.5 block text-sm font-semibold ${dash.ink}`}>
                Où avez-vous rencontré le problème ?
              </label>
              <select
                id="assistance-scope"
                className={`${dash.field} cursor-pointer`}
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
              <label htmlFor="assistance-desc" className={`mb-1.5 block text-sm font-semibold ${dash.ink}`}>
                Description du problème
              </label>
              <textarea
                id="assistance-desc"
                rows={7}
                className={`${dash.field} min-h-[140px] resize-y`}
                placeholder="Décrivez ce qui ne fonctionne pas, les étapes pour reproduire le bug, le message d'erreur éventuel…"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
              <p className={`mt-1 text-xs ${dash.textMid}`}>Minimum 15 caractères.</p>
            </div>

            <div>
              <label className={`mb-1.5 block text-sm font-semibold ${dash.ink}`}>
                Pièces jointes (optionnel)
              </label>
              <p className={`mb-2 text-xs ${dash.textMid}`}>
                PDF, photos (JPG, PNG…) — {MAX_FILES} fichiers max, 12 Mo chacun.
              </p>
              <input
                ref={fileRef}
                type="file"
                multiple
                accept="image/*,application/pdf,.pdf,.doc,.docx,.xls,.xlsx"
                className={`block w-full text-sm ${dash.textMid} file:mr-3 file:rounded-lg file:border-0 file:bg-[color:var(--dash-soft)] file:px-3 file:py-2 file:font-semibold file:text-[var(--dash-primary)]`}
                onChange={(e) => onPickFiles(e.target.files)}
              />
              {files.length > 0 && (
                <ul className="mt-3 space-y-2">
                  {files.map((f, i) => (
                    <li
                      key={`${f.name}-${i}`}
                      className={`flex items-center justify-between gap-2 rounded-lg px-3 py-2 text-sm ${dash.bgSoft50}`}
                    >
                      <span className={`truncate ${dash.ink}`}>{f.name}</span>
                      <button
                        type="button"
                        onClick={() => removeFile(i)}
                        className="shrink-0 cursor-pointer text-xs font-semibold text-rose-700 hover:underline"
                      >
                        Retirer
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            {error && (
              <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                {error}
              </p>
            )}

            {success && (
              <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
                {success}
              </p>
            )}

            <ModuleButton disabled={!canSubmit} onClick={() => void submit()} className="w-full px-6 py-3 sm:w-auto">
              {submitting ? "Envoi en cours…" : "Envoyer la demande"}
            </ModuleButton>
          </>
        )}
      </ModuleCard>
    </ModulePageShell>
  );
}
