"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { InternatRollCall, InternatRollMark, InternatStudent } from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";
import { todayDateParis } from "@/app/lib/internat-stats";

const MARKS: { id: InternatRollMark; label: string; cls: string }[] = [
  { id: "present", label: "Présent", cls: "bg-emerald-100 text-emerald-800 border-emerald-200" },
  { id: "activite", label: "Activité ext.", cls: "bg-sky-100 text-sky-800 border-sky-200" },
  { id: "absent", label: "Absent", cls: "bg-red-100 text-red-800 border-red-200" },
  { id: "excuse", label: "Excusé", cls: "bg-amber-100 text-amber-800 border-amber-200" },
];

type SaveStatus = "idle" | "saving" | "saved" | "error";

export default function InternatRollCallPanel({ onRefresh }: { onRefresh: () => Promise<void> }) {
  const [date, setDate] = useState(todayDateParis());
  const [period, setPeriod] = useState<"matin" | "soir">("soir");
  const [rollCall, setRollCall] = useState<InternatRollCall | null>(null);
  const [students, setStudents] = useState<InternatStudent[]>([]);
  const [photoUrls, setPhotoUrls] = useState<Record<string, string>>({});
  const [canValidate, setCanValidate] = useState(false);
  const [boysComplete, setBoysComplete] = useState(false);
  const [girlsComplete, setGirlsComplete] = useState(false);
  const [section, setSection] = useState<"boys" | "girls">("boys");
  const [busy, setBusy] = useState(false);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [savingStudentId, setSavingStudentId] = useState<string | null>(null);
  const saveSeq = useRef(0);
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const load = useCallback(async () => {
    const res = await fetch(`/api/internat/roll-call?date=${encodeURIComponent(date)}&period=${period}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Chargement impossible");
    setRollCall(data.rollCall);
    setStudents(data.students || []);
    setPhotoUrls(
      data.photoUrls && typeof data.photoUrls === "object" ? (data.photoUrls as Record<string, string>) : {},
    );
    setCanValidate(!!data.canValidate);
    setBoysComplete(!!data.boysComplete);
    setGirlsComplete(!!data.girlsComplete);
  }, [date, period]);

  useEffect(() => {
    void load().catch((e) => alert(e instanceof Error ? e.message : "Erreur"));
  }, [load]);

  const applyRollCallResponse = (data: {
    rollCall: InternatRollCall;
    canValidate?: boolean;
    boysComplete?: boolean;
    girlsComplete?: boolean;
  }) => {
    setRollCall(data.rollCall);
    if (data.canValidate !== undefined) setCanValidate(!!data.canValidate);
    if (data.boysComplete !== undefined) setBoysComplete(!!data.boysComplete);
    if (data.girlsComplete !== undefined) setGirlsComplete(!!data.girlsComplete);
  };

  const patchSection = async (
    payload: Record<string, unknown>,
    options?: { optimisticRollCall?: InternatRollCall; studentId?: string },
  ) => {
    const seq = ++saveSeq.current;
    if (options?.studentId) setSavingStudentId(options.studentId);
    setSaveStatus("saving");

    try {
      const res = await fetch("/api/internat/roll-call", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ date, section, period, ...payload }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      if (seq === saveSeq.current) {
        applyRollCallResponse(data);
        setSaveStatus("saved");
        if (savedTimer.current) clearTimeout(savedTimer.current);
        savedTimer.current = setTimeout(() => setSaveStatus("idle"), 2500);
      }
    } catch (e: unknown) {
      if (seq === saveSeq.current) {
        setSaveStatus("error");
        if (options?.optimisticRollCall) setRollCall(options.optimisticRollCall);
        alert(e instanceof Error ? e.message : "Erreur");
      }
    } finally {
      if (options?.studentId) setSavingStudentId(null);
    }
  };

  const setMark = (studentId: string, mark: InternatRollMark) => {
    if (!rollCall || rollCall.status === "validee") return;

    const currentMark = rollCall[section].marks[studentId];
    const nextValue: InternatRollMark | null = currentMark === mark ? null : mark;

    const optimistic: InternatRollCall = {
      ...rollCall,
      [section]: {
        ...rollCall[section],
        completed: false,
        completedBy: undefined,
        completedAt: undefined,
        marks: { ...rollCall[section].marks },
      },
      updatedAt: new Date().toISOString(),
    };
    if (nextValue === null) delete optimistic[section].marks[studentId];
    else optimistic[section].marks[studentId] = nextValue;

    setRollCall(optimistic);
    if (nextValue === null) {
      void patchSection({ marks: { [studentId]: null } }, { optimisticRollCall: rollCall, studentId });
    } else {
      void patchSection({ marks: { [studentId]: nextValue } }, { optimisticRollCall: rollCall, studentId });
    }
  };

  const completeSection = () => {
    if (!confirm(`Terminer la section ${section === "boys" ? "garçons" : "filles"} ?`)) return;
    setBusy(true);
    void patchSection({ complete: true }).finally(() => setBusy(false));
  };

  const validate = async () => {
    if (!confirm("Valider l'appel et envoyer le mail aux destinataires configurés ?")) return;
    setBusy(true);
    try {
      const res = await fetch("/api/internat/roll-call", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "validate", date, period }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Validation impossible");
      setRollCall(data.rollCall);
      setCanValidate(false);
      await onRefresh();
      alert(data.mail?.sent ? "Appel validé et mail envoyé." : "Appel validé (mail non envoyé — vérifiez SMTP / destinataires).");
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const sexe = section === "boys" ? "M" : "F";
  const list = students.filter((s) => s.actif && s.sexe === sexe);
  const sectionData = rollCall ? rollCall[section] : null;
  const locked = rollCall?.status === "validee";

  const saveLabel =
    saveStatus === "saving"
      ? "Enregistrement…"
      : saveStatus === "saved"
        ? "Enregistré sur le serveur"
        : saveStatus === "error"
          ? "Erreur de sauvegarde"
          : rollCall?.updatedAt
            ? `Dernière sauvegarde : ${new Date(rollCall.updatedAt).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`
            : null;

  return (
    <div className="space-y-5">
      <div className="rounded-xl border border-emerald-100 bg-emerald-50/60 px-4 py-3 text-sm text-emerald-900">
        Chaque marquage est <strong>enregistré immédiatement</strong> — vous pouvez quitter la page et reprendre plus tard.
        Recliquer sur un statut déjà sélectionné l&apos;efface.
      </div>

      <div className="flex flex-wrap gap-3 items-center justify-between">
        <div className="flex flex-wrap gap-3 items-center">
          <label className="text-sm font-bold text-slate-600">
            Période
            <select
              className="ml-2 border rounded-xl px-3 py-2"
              value={period}
              onChange={(e) => setPeriod(e.target.value as "matin" | "soir")}
            >
              <option value="soir">Appel du soir</option>
              <option value="matin">Appel du matin</option>
            </select>
          </label>
          <label className="text-sm font-bold text-slate-600">
            Date
            <input
              type="date"
              className="ml-2 border rounded-xl px-3 py-2"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </label>
          <span
            className={`text-xs font-bold px-3 py-1 rounded-full ${
              locked ? "bg-emerald-100 text-emerald-800" : "bg-slate-100 text-slate-600"
            }`}
          >
            {locked ? "Validé" : "Ouvert"}
          </span>
        </div>
        {saveLabel && (
          <span
            className={`text-xs font-semibold ${
              saveStatus === "error" ? "text-red-600" : saveStatus === "saving" ? "text-amber-700" : "text-slate-500"
            }`}
          >
            {saveLabel}
          </span>
        )}
      </div>

      <div className="flex gap-2 p-1 bg-slate-100 rounded-xl w-fit">
        <button
          type="button"
          onClick={() => setSection("boys")}
          className={`px-4 py-2 rounded-lg text-sm font-bold ${section === "boys" ? "bg-white shadow" : ""}`}
        >
          Garçons {boysComplete ? "✓" : ""}
        </button>
        <button
          type="button"
          onClick={() => setSection("girls")}
          className={`px-4 py-2 rounded-lg text-sm font-bold ${section === "girls" ? "bg-white shadow" : ""}`}
        >
          Filles {girlsComplete ? "✓" : ""}
        </button>
      </div>

      {list.length === 0 ? (
        <p className="text-slate-500 text-sm">Aucun interne actif pour cette section.</p>
      ) : (
        <ul className="space-y-2">
          {list.map((s) => {
            const mark = sectionData?.marks[s.id];
            const isSaving = savingStudentId === s.id;
            const photo = photoUrls[s.id];
            const initials = `${s.eleveRef.prenom?.[0] ?? ""}${s.eleveRef.nom?.[0] ?? ""}`.toUpperCase();
            return (
              <li
                key={s.id}
                className={`bg-white border rounded-xl p-3 flex flex-wrap items-center justify-between gap-2 transition-opacity ${
                  isSaving ? "border-indigo-200 opacity-80" : "border-slate-200"
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className="h-12 w-12 shrink-0 overflow-hidden rounded-full bg-slate-100 border border-slate-200">
                    {photo ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img src={photo} alt="" className="h-full w-full object-cover" />
                    ) : (
                      <span className="flex h-full w-full items-center justify-center text-xs font-bold text-slate-500">
                        {initials || "?"}
                      </span>
                    )}
                  </div>
                  <div className="min-w-0">
                    <p className="font-bold text-slate-900 truncate">{studentDisplayName(s)}</p>
                    <p className="text-xs text-slate-500">
                      {s.classe} · {s.etablissement}
                      {!mark && !locked && <span className="text-slate-400"> · non marqué</span>}
                    </p>
                  </div>
                </div>
                <div className="flex gap-1">
                  {MARKS.map((m) => (
                    <button
                      key={m.id}
                      type="button"
                      disabled={locked}
                      onClick={() => setMark(s.id, m.id)}
                      className={`px-2.5 py-1 rounded-lg text-xs font-bold border ${
                        mark === m.id ? m.cls : "bg-white text-slate-500 border-slate-200"
                      }`}
                    >
                      {m.label}
                    </button>
                  ))}
                </div>
              </li>
            );
          })}
        </ul>
      )}

      {!locked && list.length > 0 && !sectionData?.completed && (
        <button
          type="button"
          disabled={busy || saveStatus === "saving"}
          onClick={completeSection}
          className="bg-slate-800 text-white px-5 py-2.5 rounded-xl font-bold text-sm disabled:opacity-40"
        >
          Terminer la section {section === "boys" ? "garçons" : "filles"}
        </button>
      )}

      {sectionData?.completed && (
        <p className="text-sm text-emerald-700 font-semibold">
          Section complétée{sectionData.completedBy ? ` par ${sectionData.completedBy}` : ""}.
        </p>
      )}

      {!locked && (
        <button
          type="button"
          disabled={!canValidate || busy || saveStatus === "saving"}
          onClick={validate}
          className="bg-indigo-600 text-white px-5 py-2.5 rounded-xl font-bold text-sm disabled:opacity-40"
        >
          Finaliser l&apos;appel et envoyer le mail
        </button>
      )}
    </div>
  );
}
