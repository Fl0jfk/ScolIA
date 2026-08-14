"use client";

import { useRef, useState } from "react";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";

type ManualFormState = {
  firstName: string;
  lastName: string;
  examType: string;
  scope: "professeur" | "ogec";
  etablissement: string;
  startDate: string;
  endDate: string;
  startTime: string;
  endTime: string;
};

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

function localDateInputValue(d: Date) {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

type Props = {
  onSuccess?: () => void;
};

export default function AbsencesDeclareOther({ onSuccess }: Props) {
  const manualPdfRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [savingManual, setSavingManual] = useState(false);
  const [manualForm, setManualForm] = useState<ManualFormState>(() => {
    const t = localDateInputValue(new Date());
    return {
      firstName: "",
      lastName: "",
      examType: "",
      scope: "professeur",
      etablissement: "",
      startDate: t,
      endDate: t,
      startTime: "08:00",
      endTime: "18:00",
    };
  });
  const [dragActive, setDragActive] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [ingestPhase, setIngestPhase] = useState<string | null>(null);

  const pollIngest = async (jobId: string) => {
    const deadline = Date.now() + 10 * 60 * 1000;
    let polls = 0;
    while (Date.now() < deadline) {
      if (polls > 0) await new Promise((r) => setTimeout(r, polls < 3 ? 1500 : 3000));
      polls += 1;
      const procRes = await fetch("/api/absences/ingest/process", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jobId }),
      });
      if (procRes.status === 401 || procRes.status === 403) {
        const t = await procRes.text().catch(() => "");
        let err = "";
        try {
          err = JSON.parse(t).error || "";
        } catch {
          err = t.slice(0, 120);
        }
        throw new Error(err || `Import refusé (${procRes.status}).`);
      }
      if (!procRes.ok && procRes.status !== 202 && procRes.status !== 200) {
        throw new Error(`Impossible de lancer l'analyse (${procRes.status}). Réessayez.`);
      }
      const stRes = await fetch(`/api/absences/ingest/status?jobId=${encodeURIComponent(jobId)}`);
      const stRaw = await stRes.text();
      let st: { status?: string; phase?: string | null; error?: string; created?: { id: string }[] } = {};
      try {
        st = stRaw ? (JSON.parse(stRaw) as typeof st) : {};
      } catch {
        throw new Error("Réponse serveur invalide pendant l'analyse.");
      }
      if (!stRes.ok) throw new Error(st.error || `Suivi d'import impossible (${stRes.status}).`);
      if (st.phase === "ocr") setIngestPhase("Lecture du PDF (OCR)…");
      else if (st.phase === "ai") setIngestPhase("Analyse des dates et du professeur (IA)…");
      else if (st.phase === "saving") setIngestPhase("Enregistrement des absences…");
      else if (st.status === "pending" || st.status === "processing") setIngestPhase("Analyse en cours…");
      if (st.status === "completed") {
        setIngestPhase(null);
        return st.created?.length || 0;
      }
      if (st.status === "failed") {
        setIngestPhase(null);
        throw new Error(st.error || "Analyse du PDF échouée.");
      }
    }
    setIngestPhase(null);
    throw new Error(
      "L'analyse dépasse 10 minutes (file d'attente OCR chargée). Réessayez dans un instant ou saisissez l'absence à la main.",
    );
  };

  const handleUpload = async (file: File) => {
    setError(null);
    setSuccess(null);
    const name = String(file.name || "").toLowerCase();
    const isPdf = name.endsWith(".pdf") || file.type === "application/pdf" || file.type === "application/x-pdf";
    if (!isPdf) {
      setError("Seuls les fichiers PDF (.pdf) sont acceptés.");
      return;
    }
    if (file.size === 0) {
      setError("Le fichier est vide.");
      return;
    }
    if (file.size > 15 * 1024 * 1024) {
      setError("Le PDF dépasse 15 Mo.");
      return;
    }
    const formData = new FormData();
    formData.append("file", file, file.name || "convocation.pdf");
    try {
      setUploading(true);
      const res = await fetch("/api/absences/ingest", { method: "POST", body: formData });
      const raw = await res.text();
      let payload: {
        error?: string;
        detail?: string;
        created?: unknown[];
        jobId?: string;
        accepted?: boolean;
      } = {};
      try {
        payload = raw ? (JSON.parse(raw) as typeof payload) : {};
      } catch {
        payload = { error: raw.slice(0, 240) || undefined };
      }
      if (res.status === 202 && payload.accepted && payload.jobId) {
        const n = await pollIngest(payload.jobId);
        setSuccess(
          n <= 1
            ? `Import réussi : ${n} créneau enregistré.`
            : `Import réussi : ${n} créneaux enregistrés (même convocation).`,
        );
        onSuccess?.();
        return;
      }
      if (!res.ok) throw new Error(payload?.error || `Import échoué (erreur ${res.status}).`);
      const n = payload?.created?.length || 0;
      setSuccess(n <= 1 ? `Import réussi : ${n} créneau enregistré.` : `Import réussi : ${n} créneaux enregistrés.`);
      onSuccess?.();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur pendant l'import.");
    } finally {
      setUploading(false);
      setIngestPhase(null);
    }
  };

  const submitManualAbsence = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(null);
    const fd = new FormData();
    fd.append("firstName", manualForm.firstName.trim());
    fd.append("lastName", manualForm.lastName.trim());
    fd.append("examType", manualForm.examType.trim());
    fd.append("scope", manualForm.scope);
    if (manualForm.scope === "professeur") {
      fd.append("etablissement", manualForm.etablissement);
    }
    fd.append("startDate", manualForm.startDate);
    fd.append("endDate", manualForm.endDate);
    fd.append("startTime", manualForm.startTime);
    fd.append("endTime", manualForm.endTime);
    const pdf = manualPdfRef.current?.files?.[0];
    if (pdf) fd.append("justificatif", pdf);
    try {
      setSavingManual(true);
      const res = await fetch("/api/absences/manual", { method: "POST", body: fd });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Enregistrement impossible.");
      setSuccess("Absence enregistrée (saisie manuelle).");
      const t = localDateInputValue(new Date());
      setManualForm((prev) => ({
        ...prev,
        firstName: "",
        lastName: "",
        examType: "",
        startDate: t,
        endDate: t,
      }));
      if (manualPdfRef.current) manualPdfRef.current.value = "";
      onSuccess?.();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur pendant la saisie manuelle.");
    } finally {
      setSavingManual(false);
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <section className="bg-white border border-slate-200 rounded-3xl p-6">
        <h2 className="text-xl font-black text-slate-900 mb-2">Saisie manuelle</h2>
        <p className="text-sm text-slate-500 mb-4">
          Pour un professeur ou un membre du personnel dont l&apos;absence n&apos;est pas la vôtre. Utile si l&apos;import PDF ne reconnaît pas le nom ou les dates.
        </p>
        <form onSubmit={submitManualAbsence} className="space-y-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label className="block">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Prénom</span>
              <input
                required
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={manualForm.firstName}
                onChange={(e) => setManualForm((p) => ({ ...p, firstName: e.target.value }))}
                disabled={savingManual}
                autoComplete="given-name"
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Nom</span>
              <input
                required
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={manualForm.lastName}
                onChange={(e) => setManualForm((p) => ({ ...p, lastName: e.target.value }))}
                disabled={savingManual}
                autoComplete="family-name"
              />
            </label>
          </div>
          <label className="block">
            <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Motif / type d&apos;absence</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              placeholder="Ex. convocation bac, formation, arrêt maladie…"
              value={manualForm.examType}
              onChange={(e) => setManualForm((p) => ({ ...p, examType: e.target.value }))}
              disabled={savingManual}
            />
          </label>
          <label className="block max-w-xs">
            <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Type</span>
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-semibold text-slate-800"
              value={manualForm.scope}
              onChange={(e) =>
                setManualForm((p) => ({
                  ...p,
                  scope: e.target.value === "ogec" ? "ogec" : "professeur",
                }))
              }
              disabled={savingManual}
            >
              <option value="professeur">Professeur</option>
              <option value="ogec">Personnel OGEC</option>
            </select>
          </label>
          {manualForm.scope === "professeur" && (
            <label className="block max-w-xs">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Établissement</span>
              <EstablishmentSelect
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-semibold text-slate-800"
                value={manualForm.etablissement}
                onChange={(label) => setManualForm((p) => ({ ...p, etablissement: label }))}
                required
                disabled={savingManual}
              />
            </label>
          )}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label className="block">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Du (jour)</span>
              <input
                required
                type="date"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={manualForm.startDate}
                onChange={(e) => setManualForm((p) => ({ ...p, startDate: e.target.value }))}
                disabled={savingManual}
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Au (jour)</span>
              <input
                required
                type="date"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={manualForm.endDate}
                onChange={(e) => setManualForm((p) => ({ ...p, endDate: e.target.value }))}
                disabled={savingManual}
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Heure de début</span>
              <input
                required
                type="time"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={manualForm.startTime}
                onChange={(e) => setManualForm((p) => ({ ...p, startTime: e.target.value }))}
                disabled={savingManual}
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Heure de fin</span>
              <input
                required
                type="time"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={manualForm.endTime}
                onChange={(e) => setManualForm((p) => ({ ...p, endTime: e.target.value }))}
                disabled={savingManual}
              />
            </label>
          </div>
          <label className="block">
            <span className="text-xs font-bold text-slate-600 uppercase tracking-wide">Justificatif PDF (optionnel)</span>
            <input
              ref={manualPdfRef}
              type="file"
              accept="application/pdf"
              className="mt-1 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border file:border-slate-200 file:bg-white file:px-3 file:py-2 file:text-sm file:font-semibold"
              disabled={savingManual}
            />
          </label>
          <button
            type="submit"
            disabled={savingManual || uploading}
            className="rounded-xl bg-slate-900 px-4 py-2.5 text-sm font-bold text-white hover:bg-slate-800 disabled:opacity-60"
          >
            {savingManual ? "Enregistrement…" : "Enregistrer l'absence"}
          </button>
        </form>
      </section>

      <section className="bg-white border border-slate-200 rounded-3xl p-6">
        <h2 className="text-xl font-black text-slate-900 mb-2">Import PDF (OCR + IA)</h2>
        <p className="text-sm text-slate-500 mb-4">
          Glissez une convocation ou un justificatif PDF : le nom, les dates et les créneaux sont détectés automatiquement.
        </p>
        <label
          className={[
            "block border-2 border-dashed rounded-2xl p-8 text-center cursor-pointer transition-colors",
            dragActive ? "border-indigo-500 bg-indigo-50/60" : "border-slate-300 hover:border-indigo-400 hover:bg-indigo-50/40",
            uploading || savingManual ? "opacity-70 cursor-not-allowed" : "",
          ].join(" ")}
          onDragEnter={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (!uploading && !savingManual) setDragActive(true);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (!uploading && !savingManual) setDragActive(true);
          }}
          onDragLeave={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setDragActive(false);
          }}
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setDragActive(false);
            if (uploading || savingManual) return;
            const file = e.dataTransfer.files?.[0];
            if (!file) {
              setError("Aucun fichier détecté dans le glisser-déposer.");
              return;
            }
            handleUpload(file);
          }}
        >
          <input
            type="file"
            accept="application/pdf"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) handleUpload(file);
              e.currentTarget.value = "";
            }}
            disabled={uploading || savingManual}
          />
          <p className="text-sm font-semibold text-slate-700">
            {uploading
              ? ingestPhase || "Analyse du PDF en cours — ne fermez pas la page…"
              : savingManual
                ? "Enregistrement manuel en cours…"
                : "Glissez-déposez un PDF ici, ou cliquez pour sélectionner"}
          </p>
        </label>
      </section>

      {success && (
        <p className="text-sm text-emerald-700 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">{success}</p>
      )}
      {error && (
        <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">{error}</p>
      )}
    </div>
  );
}
