"use client";

import { useCallback, useEffect, useState } from "react";
import { TripDocumentsDropZone } from "@/app/components/travels/TripDocumentsDropZone";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { notifyTravelsAssistanceCardChanged } from "@/app/hooks/useTravelsAssistanceCard";

type AssistanceCardState = {
  configured: boolean;
  fileName: string | null;
  downloadUrl: string | null;
};

/** Paramétrage PDF carte d’assistance (mutuelle) — admin établissement. */
export default function TravelsAssistanceCardSettingsPanel({
  compact = false,
}: {
  compact?: boolean;
}) {
  const [state, setState] = useState<AssistanceCardState>({
    configured: false,
    fileName: null,
    downloadUrl: null,
  });
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings/upload-assistance-card", { cache: "no-store" });
      const j = (await res.json()) as AssistanceCardState & { error?: string };
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setState({
        configured: Boolean(j.configured),
        fileName: j.fileName ?? null,
        downloadUrl: j.downloadUrl ?? null,
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const uploadPdf = async (files: File[]) => {
    const file = files[0];
    if (!file) return;
    if (file.type !== "application/pdf" && !file.name.toLowerCase().endsWith(".pdf")) {
      setError("Seuls les fichiers PDF sont acceptés.");
      return;
    }
    setUploading(true);
    setMsg(null);
    setError(null);
    try {
      const prep = await fetch("/api/settings/upload-assistance-card", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileName: file.name, fileType: "application/pdf" }),
      });
      const prepJson = await prep.json();
      if (!prep.ok) throw new Error(prepJson.error || "Préparation upload impossible");

      const putRes = await fetch(prepJson.uploadUrl as string, {
        method: "PUT",
        body: file,
        headers: { "Content-Type": "application/pdf" },
      });
      if (!putRes.ok) throw new Error("Envoi du PDF sur le stockage sécurisé impossible.");

      setState({
        configured: true,
        fileName: (prepJson.fileName as string) || file.name,
        downloadUrl: (prepJson.downloadUrl as string) || "/api/travels/assistance-card?raw=1",
      });
      setMsg("Carte d’assistance enregistrée.");
      notifyTravelsAssistanceCardChanged();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur upload");
    } finally {
      setUploading(false);
    }
  };

  const removePdf = async () => {
    if (!state.configured) return;
    if (!window.confirm("Retirer la carte d’assistance pour tout l’établissement ?")) return;
    setRemoving(true);
    setMsg(null);
    setError(null);
    try {
      const res = await fetch("/api/settings/upload-assistance-card", { method: "DELETE" });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Suppression impossible");
      setState({ configured: false, fileName: null, downloadUrl: null });
      setMsg("Carte d’assistance retirée.");
      notifyTravelsAssistanceCardChanged();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setRemoving(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement carte d’assistance…</p>;
  }

  const shellClass = compact
    ? "space-y-3"
    : "rounded-2xl border border-slate-200 bg-white p-6 space-y-4 max-w-5xl";

  return (
    <div className={shellClass}>
      {!compact && (
        <div>
          <h2 className="text-lg font-black text-slate-900">Carte d’assistance</h2>
          <p className="mt-1 text-sm text-slate-600">
            Contrat et numéros d’urgence mutuelle — un PDF pour tout l’établissement, visible sur
            chaque dossier de sortie / voyage.
          </p>
        </div>
      )}
      {compact && (
        <p className="text-sm font-bold text-slate-800">Carte d’assistance (PDF établissement)</p>
      )}
      {error && <p className="text-sm text-rose-700">{error}</p>}
      {msg && <p className="text-sm text-emerald-700">{msg}</p>}

      {state.configured && state.fileName ? (
        <div className="flex flex-wrap items-center gap-3 rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
          <span className="font-semibold text-slate-800">📄 {state.fileName}</span>
          {state.downloadUrl ? (
            <a
              href={state.downloadUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-bold text-indigo-600 hover:underline"
            >
              Ouvrir
            </a>
          ) : null}
          <ModuleButton
            variant="secondary"
            disabled={removing || uploading}
            onClick={() => void removePdf()}
          >
            Retirer
          </ModuleButton>
        </div>
      ) : null}

      <TripDocumentsDropZone
        title="Carte d’assistance — déposer un PDF"
        hint={
          state.configured
            ? "Glisser un nouveau PDF pour remplacer le fichier actuel"
            : "Glisser-déposer ou cliquer — un seul PDF pour l’établissement"
        }
        multiple={false}
        uploading={uploading}
        disabled={removing}
        onFiles={uploadPdf}
        className="py-6"
      />
    </div>
  );
}
