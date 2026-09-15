"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { InternatRollCall, InternatRollMark, InternatStudent } from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";
import { isInternatEveningRollCallDay, todayDateParis } from "@/app/lib/internat-stats";
import {
  INTERNAT_NIVEAUX,
  niveauDisplayLabel,
  niveauFromClasse,
  niveauSortKey,
} from "@/app/lib/internat-level";

const PRIMARY_MARKS: { id: InternatRollMark; label: string; activeCls: string }[] = [
  {
    id: "present",
    label: "Présent",
    activeCls: "bg-emerald-600 text-white border-emerald-600",
  },
  {
    id: "absent",
    label: "Absent",
    activeCls: "bg-red-600 text-white border-red-600",
  },
];

function askMarkNote(params: {
  mark: InternatRollMark;
  afterValidation: boolean;
  previous?: InternatRollMark;
}): string | null | undefined {
  // Activité = en attente, motif par défaut sans bloquer le geste.
  if (params.mark === "activite" && !params.afterValidation) {
    return "Activité sportive / extérieure";
  }
  const needsNote =
    params.afterValidation ||
    (params.previous === "activite" && (params.mark === "present" || params.mark === "excuse")) ||
    (params.previous === "absent" && (params.mark === "present" || params.mark === "excuse"));
  if (!needsNote) return undefined;
  const hint =
    params.previous === "activite" && params.mark === "present"
      ? "Retour d’activité — préciser si besoin (ex. arrivé 21h10). Laisser vide OK."
      : params.afterValidation
        ? "Motif obligatoire (ex. activité sportive — arrivé à 21h10)."
        : "Motif (optionnel).";
  const raw = window.prompt(hint, params.previous === "activite" ? "Retour d’activité" : "");
  if (raw === null) return null;
  const note = raw.trim();
  if (params.afterValidation && !note) {
    alert("Un motif est requis pour corriger un appel déjà validé.");
    return null;
  }
  return note || undefined;
}

type CourseAbsenceHint = {
  absenceId: string;
  eleveId: string;
  type: "absence" | "retard";
  justifie: boolean;
  statut: string;
  motif: string | null;
  label: string;
};

type SaveStatus = "idle" | "saving" | "saved" | "error";

function sectionKey(sexe: "M" | "F"): "boys" | "girls" {
  return sexe === "F" ? "girls" : "boys";
}

function getMark(rollCall: InternatRollCall | null, student: InternatStudent): InternatRollMark | undefined {
  if (!rollCall) return undefined;
  return rollCall[sectionKey(student.sexe)].marks[student.id];
}

function getMarkMeta(rollCall: InternatRollCall | null, student: InternatStudent) {
  if (!rollCall) return undefined;
  return rollCall[sectionKey(student.sexe)].markMeta?.[student.id];
}

function formatMarkTime(iso: string | undefined): string | null {
  if (!iso) return null;
  try {
    return new Date(iso).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  } catch {
    return null;
  }
}

export default function InternatRollCallPanel({ onRefresh }: { onRefresh: () => Promise<void> }) {
  const [date, setDate] = useState(todayDateParis());
  const [period, setPeriod] = useState<"matin" | "soir">("soir");
  const [rollCall, setRollCall] = useState<InternatRollCall | null>(null);
  const [students, setStudents] = useState<InternatStudent[]>([]);
  const [photoUrls, setPhotoUrls] = useState<Record<string, string>>({});
  const [courseAbsenceHints, setCourseAbsenceHints] = useState<Record<string, CourseAbsenceHint>>({});
  const [canValidate, setCanValidate] = useState(false);
  const [boysComplete, setBoysComplete] = useState(false);
  const [girlsComplete, setGirlsComplete] = useState(false);
  const [busy, setBusy] = useState(false);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [savingStudentId, setSavingStudentId] = useState<string | null>(null);
  const [filterSexe, setFilterSexe] = useState<"all" | "M" | "F">("all");
  const [filterEtab, setFilterEtab] = useState<string>("all");
  const [filterNiveau, setFilterNiveau] = useState<string>("all");
  const [viewerScope, setViewerScope] = useState<"all" | "college" | "lycee">("all");
  const [pendingActivityCount, setPendingActivityCount] = useState(0);
  const [pendingActivityNames, setPendingActivityNames] = useState<string[]>([]);
  const saveSeq = useRef(0);
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const load = useCallback(async () => {
    const res = await fetch(
      `/api/internat/roll-call?date=${encodeURIComponent(date)}&period=${period}`,
      { cache: "no-store" },
    );
    const raw = await res.text();
    let data: {
      error?: string;
      rollCall?: InternatRollCall;
      students?: InternatStudent[];
      photoUrls?: Record<string, string>;
      courseAbsenceHints?: Record<string, CourseAbsenceHint>;
      canValidate?: boolean;
      boysComplete?: boolean;
      girlsComplete?: boolean;
      viewerScope?: "all" | "college" | "lycee";
      pendingActivityCount?: number;
      pendingActivityNames?: string[];
    } = {};
    try {
      data = raw ? (JSON.parse(raw) as typeof data) : {};
    } catch {
      throw new Error(res.ok ? "Réponse invalide" : `Erreur serveur (${res.status})`);
    }
    if (!res.ok) throw new Error(data?.error || `Chargement impossible (${res.status})`);
    setRollCall(data.rollCall ?? null);
    setStudents(data.students || []);
    setViewerScope(data.viewerScope === "college" || data.viewerScope === "lycee" ? data.viewerScope : "all");
    setPendingActivityCount(Number(data.pendingActivityCount) || 0);
    setPendingActivityNames(Array.isArray(data.pendingActivityNames) ? data.pendingActivityNames : []);
    setPhotoUrls(
      data.photoUrls && typeof data.photoUrls === "object" ? data.photoUrls : {},
    );
    setCourseAbsenceHints(
      data.courseAbsenceHints && typeof data.courseAbsenceHints === "object"
        ? data.courseAbsenceHints
        : {},
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
    pendingActivityCount?: number;
    pendingActivityNames?: string[];
  }) => {
    setRollCall(data.rollCall);
    if (data.canValidate !== undefined) setCanValidate(!!data.canValidate);
    if (data.boysComplete !== undefined) setBoysComplete(!!data.boysComplete);
    if (data.girlsComplete !== undefined) setGirlsComplete(!!data.girlsComplete);
    if (data.pendingActivityCount !== undefined) {
      setPendingActivityCount(Number(data.pendingActivityCount) || 0);
    }
    if (data.pendingActivityNames !== undefined) {
      setPendingActivityNames(Array.isArray(data.pendingActivityNames) ? data.pendingActivityNames : []);
    }
  };

  const patchSection = async (
    section: "boys" | "girls",
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

  const setMark = (student: InternatStudent, mark: InternatRollMark) => {
    if (!rollCall) return;

    const key = sectionKey(student.sexe);
    const currentMark = rollCall[key].marks[student.id];
    const afterValidation = rollCall.status === "validee";
    const nextValue: InternatRollMark | null = !afterValidation && currentMark === mark ? null : mark;

    if (nextValue === null) {
      const optimistic: InternatRollCall = {
        ...rollCall,
        [key]: {
          ...rollCall[key],
          completed: false,
          completedBy: undefined,
          completedAt: undefined,
          marks: { ...rollCall[key].marks },
          markMeta: { ...(rollCall[key].markMeta || {}) },
        },
        updatedAt: new Date().toISOString(),
      };
      delete optimistic[key].marks[student.id];
      if (optimistic[key].markMeta) {
        delete optimistic[key].markMeta[student.id];
      }
      setRollCall(optimistic);
      void patchSection(
        key,
        { marks: { [student.id]: null } },
        { optimisticRollCall: rollCall, studentId: student.id },
      );
      return;
    }

    const noteResult = askMarkNote({
      mark: nextValue,
      afterValidation,
      previous: currentMark,
    });
    if (noteResult === null) return;

    const now = new Date().toISOString();
    const optimistic: InternatRollCall = {
      ...rollCall,
      [key]: {
        ...rollCall[key],
        completed: afterValidation ? rollCall[key].completed : false,
        completedBy: afterValidation ? rollCall[key].completedBy : undefined,
        completedAt: afterValidation ? rollCall[key].completedAt : undefined,
        marks: { ...rollCall[key].marks, [student.id]: nextValue },
        markMeta: {
          ...(rollCall[key].markMeta || {}),
          [student.id]: {
            at: now,
            by: "vous",
            note: noteResult,
            previous: currentMark,
          },
        },
      },
      updatedAt: now,
    };

    setRollCall(optimistic);
    void patchSection(
      key,
      { marks: { [student.id]: nextValue }, note: noteResult },
      { optimisticRollCall: rollCall, studentId: student.id },
    );
  };

  const completeSection = (section: "boys" | "girls") => {
    const label = section === "boys" ? "garçons" : "filles";
    if (!confirm(`Terminer la section ${label} ?`)) return;
    setBusy(true);
    void patchSection(section, { complete: true }).finally(() => setBusy(false));
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
      alert(
        data.mail?.sent
          ? "Appel validé. Un PDF distinct a été envoyé à la direction et au CPE de chaque établissement (collège / lycée)."
          : "Appel validé (mail non envoyé — vérifiez SMTP et les destinataires direction/CPE dans Paramètres).",
      );
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const etablissements = useMemo(() => {
    const set = new Set<string>();
    for (const s of students) {
      if (s.actif && s.etablissement) set.add(s.etablissement);
    }
    return [...set].sort((a, b) => a.localeCompare(b, "fr"));
  }, [students]);

  const filtered = useMemo(() => {
    return students
      .filter((s) => s.actif)
      .filter((s) => (filterSexe === "all" ? true : s.sexe === filterSexe))
      .filter((s) => (filterEtab === "all" ? true : s.etablissement === filterEtab))
      .filter((s) => {
        if (filterNiveau === "all") return true;
        return niveauFromClasse(s.classe) === filterNiveau;
      })
      .sort((a, b) => {
        const na = niveauSortKey(niveauFromClasse(a.classe));
        const nb = niveauSortKey(niveauFromClasse(b.classe));
        if (na !== nb) return na - nb;
        if (a.sexe !== b.sexe) return a.sexe === "F" ? -1 : 1;
        return studentDisplayName(a).localeCompare(studentDisplayName(b), "fr");
      });
  }, [students, filterSexe, filterEtab, filterNiveau]);

  const grouped = useMemo(() => {
    const map = new Map<string, InternatStudent[]>();
    for (const s of filtered) {
      const niv = niveauFromClasse(s.classe) || "Autres";
      const list = map.get(niv) || [];
      list.push(s);
      map.set(niv, list);
    }
    return [...map.entries()].sort(
      (a, b) => niveauSortKey(a[0] === "Autres" ? null : a[0]) - niveauSortKey(b[0] === "Autres" ? null : b[0]),
    );
  }, [filtered]);

  const validated = rollCall?.status === "validee";
  const markedCount = filtered.filter((s) => getMark(rollCall, s)).length;
  const eveningNotApplicable = period === "soir" && !isInternatEveningRollCallDay(date);

  const saveLabel =
    saveStatus === "saving"
      ? "Enregistrement…"
      : saveStatus === "saved"
        ? "Enregistré"
        : saveStatus === "error"
          ? "Erreur de sauvegarde"
          : rollCall?.updatedAt
            ? `Sauvé ${new Date(rollCall.updatedAt).toLocaleTimeString("fr-FR", {
                hour: "2-digit",
                minute: "2-digit",
              })}`
            : null;

  return (
    <div className="space-y-4 max-w-3xl mx-auto pb-24 sm:pb-8">
      {viewerScope !== "all" && (
        <div className="rounded-2xl border border-indigo-200 bg-indigo-50 px-4 py-3 text-sm text-indigo-950">
          <p className="font-bold">
            Consultation {viewerScope === "college" ? "collège" : "lycée"} uniquement
          </p>
          <p className="mt-0.5 text-indigo-800/90">
            Vous voyez uniquement les internes de votre établissement. L’équipe internat
            continue de faire l’appel complet au même endroit.
          </p>
        </div>
      )}
      <div className="rounded-2xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-950">
        <p className="font-bold">Présent / Absent d’abord — activité sans bloquer l’appel</p>
        <p className="mt-0.5 text-sky-900/90">
          Bouton <span className="font-semibold">Activité</span> sous chaque élève : sport / sortie
          extérieure. Vous continuez l’appel pour les autres. L’envoi à la direction reste bloqué
          tant que ces élèves ne sont pas repassés en Présent (ou Absent / Excusé) à leur retour.
        </p>
      </div>
      {pendingActivityCount > 0 && !validated && (
        <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
          <p className="font-bold">
            {pendingActivityCount} en activité — envoi direction en pause (c’est normal)
          </p>
          <p className="mt-0.5 text-amber-900/90">
            {pendingActivityNames.slice(0, 6).join(", ")}
            {pendingActivityNames.length > 6 ? "…" : ""}. Pas d’urgence : à leur retour → Présent,
            puis « Finaliser & envoyer ». Un rappel doux part une seule fois dans la soirée à
            l’équipe internat.
          </p>
        </div>
      )}
      {validated && (
        <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-950">
          <p className="font-bold">Appel validé — corrections possibles</p>
          <p className="mt-0.5 text-emerald-900/90">
            Vous pouvez encore changer un statut (arrivée tardive). Un motif sera demandé et
            l’heure de correction notée.
          </p>
        </div>
      )}
      {eveningNotApplicable && (
        <div className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
          <p className="font-bold text-slate-900">Pas d’appel du soir ce jour</p>
          <p className="mt-0.5 text-slate-600">
            L’appel du soir est prévu le lundi, mardi, mercredi et jeudi uniquement (internes
            absents le vendredi soir et le week-end).
          </p>
        </div>
      )}
      <div className="sticky top-0 z-20 -mx-1 px-1 py-2 bg-slate-50/95 backdrop-blur-sm space-y-3 border-b border-slate-200/80">
        <div className="flex flex-wrap gap-2 items-center justify-between">
          <div className="flex flex-wrap gap-2 items-center">
            <select
              className="border border-slate-200 rounded-xl px-3 py-2.5 text-sm font-bold bg-white min-h-[44px]"
              value={period}
              onChange={(e) => setPeriod(e.target.value as "matin" | "soir")}
            >
              <option value="soir">Soir</option>
              <option value="matin">Matin</option>
            </select>
            <input
              type="date"
              className="border border-slate-200 rounded-xl px-3 py-2.5 text-sm font-semibold bg-white min-h-[44px]"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
            <span
              className={`text-xs font-bold px-2.5 py-1 rounded-lg ${
                validated ? "bg-emerald-100 text-emerald-800" : "bg-slate-100 text-slate-600"
              }`}
            >
              {validated ? "Validé" : `${markedCount}/${filtered.length}`}
            </span>
          </div>
          {saveLabel && (
            <span
              className={`text-xs font-semibold ${
                saveStatus === "error"
                  ? "text-red-600"
                  : saveStatus === "saving"
                    ? "text-amber-700"
                    : "text-slate-500"
              }`}
            >
              {saveLabel}
            </span>
          )}
        </div>

        <div className="grid grid-cols-3 gap-2">
          <select
            className="border border-slate-200 rounded-xl px-2 py-2.5 text-sm bg-white min-h-[44px]"
            value={filterSexe}
            onChange={(e) => setFilterSexe(e.target.value as "all" | "M" | "F")}
          >
            <option value="all">Sexe</option>
            <option value="F">Filles</option>
            <option value="M">Garçons</option>
          </select>
          <select
            className="border border-slate-200 rounded-xl px-2 py-2.5 text-sm bg-white min-h-[44px]"
            value={filterEtab}
            onChange={(e) => setFilterEtab(e.target.value)}
          >
            <option value="all">Établ.</option>
            {etablissements.map((e) => (
              <option key={e} value={e}>
                {e}
              </option>
            ))}
          </select>
          <select
            className="border border-slate-200 rounded-xl px-2 py-2.5 text-sm bg-white min-h-[44px]"
            value={filterNiveau}
            onChange={(e) => setFilterNiveau(e.target.value)}
          >
            <option value="all">Niveau</option>
            {INTERNAT_NIVEAUX.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </div>
      </div>

      {filtered.length === 0 ? (
        <p className="text-slate-500 text-sm text-center py-10">Aucun interne pour ces filtres.</p>
      ) : (
        <div className="space-y-6">
          {grouped.map(([niveau, list]) => (
            <section key={niveau}>
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-500 mb-3 px-0.5">
                {niveauDisplayLabel(niveau === "Autres" ? null : niveau)} · {list.length}
              </h3>
              <ul className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                {list.map((s) => {
                  const mark = getMark(rollCall, s);
                  const meta = getMarkMeta(rollCall, s);
                  const metaTime = formatMarkTime(meta?.at);
                  const isSaving = savingStudentId === s.id;
                  const photo = photoUrls[s.id];
                  const courseHint = courseAbsenceHints[s.id];
                  const initials =
                    `${s.eleveRef.prenom?.[0] ?? ""}${s.eleveRef.nom?.[0] ?? ""}`.toUpperCase() ||
                    "?";
                  return (
                    <li
                      key={s.id}
                      className={`bg-white border rounded-2xl p-3 transition-opacity ${
                        isSaving
                          ? "border-slate-300 opacity-80"
                          : courseHint && !courseHint.justifie
                            ? "border-amber-300 bg-amber-50/30"
                            : mark === "present"
                              ? "border-emerald-200"
                              : mark === "absent"
                                ? "border-red-200"
                                : mark === "activite"
                                  ? "border-sky-300 bg-sky-50/40"
                                  : "border-slate-200"
                      }`}
                    >
                      <div className="flex gap-3">
                        <div className="h-16 w-16 sm:h-20 sm:w-20 shrink-0 overflow-hidden rounded-xl bg-slate-100 border border-slate-200">
                          {photo ? (
                            // eslint-disable-next-line @next/next/no-img-element
                            <img src={photo} alt="" className="h-full w-full object-cover" />
                          ) : (
                            <span className="flex h-full w-full items-center justify-center text-sm font-bold text-slate-500">
                              {initials}
                            </span>
                          )}
                        </div>
                        <div className="min-w-0 flex-1">
                          <p className="font-bold text-slate-900 truncate leading-tight">
                            {studentDisplayName(s)}
                          </p>
                          <p className="text-xs text-slate-500 mt-0.5">
                            {s.classe} · {s.etablissement}
                          </p>
                          {mark === "activite" && (
                            <p className="mt-1 text-[11px] font-bold text-sky-800">
                              En activité — en attente de retour
                              {metaTime ? ` · depuis ${metaTime}` : ""}
                            </p>
                          )}
                          {mark !== "activite" && (meta?.note || metaTime) && (
                            <p className="mt-1 text-[11px] text-slate-600">
                              {metaTime ? `${metaTime}` : null}
                              {metaTime && meta?.note ? " · " : null}
                              {meta?.note || null}
                            </p>
                          )}
                          {courseHint ? (
                            <p
                              className={`mt-1 text-[11px] font-semibold ${
                                courseHint.justifie ? "text-slate-600" : "text-amber-800"
                              }`}
                            >
                              {courseHint.label}
                            </p>
                          ) : null}

                          <div className="mt-2.5 grid grid-cols-2 gap-1.5">
                            {PRIMARY_MARKS.map((m) => (
                              <button
                                key={m.id}
                                type="button"
                                onClick={() => setMark(s, m.id)}
                                className={`min-h-[44px] rounded-xl text-sm font-bold border transition-colors ${
                                  mark === m.id
                                    ? m.activeCls
                                    : "bg-white text-slate-600 border-slate-200 active:bg-slate-50"
                                }`}
                              >
                                {m.label}
                              </button>
                            ))}
                          </div>

                          <div className="mt-1.5 grid grid-cols-2 gap-1.5">
                            <button
                              type="button"
                              onClick={() => setMark(s, "activite")}
                              className={`min-h-[36px] rounded-lg text-xs font-bold border ${
                                mark === "activite"
                                  ? "bg-sky-600 text-white border-sky-600"
                                  : "bg-white text-sky-800 border-sky-200"
                              }`}
                            >
                              Activité
                            </button>
                            <button
                              type="button"
                              onClick={() => setMark(s, "excuse")}
                              className={`min-h-[36px] rounded-lg text-xs font-bold border ${
                                mark === "excuse"
                                  ? "bg-amber-500 text-white border-amber-500"
                                  : "bg-white text-slate-500 border-slate-200"
                              }`}
                            >
                              Excusé
                            </button>
                          </div>
                        </div>
                      </div>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
        </div>
      )}

      {!validated && (
        <div className="fixed bottom-0 inset-x-0 sm:static sm:inset-auto z-30 bg-white/95 backdrop-blur border-t border-slate-200 sm:border-0 p-3 sm:p-0 sm:pt-4 space-y-2">
          <div className="max-w-3xl mx-auto flex flex-col sm:flex-row gap-2">
            {!boysComplete && (
              <button
                type="button"
                disabled={busy || saveStatus === "saving"}
                onClick={() => completeSection("boys")}
                className="flex-1 min-h-[48px] bg-slate-800 text-white px-4 rounded-xl font-bold text-sm disabled:opacity-40"
              >
                Terminer garçons {boysComplete ? "✓" : ""}
              </button>
            )}
            {!girlsComplete && (
              <button
                type="button"
                disabled={busy || saveStatus === "saving"}
                onClick={() => completeSection("girls")}
                className="flex-1 min-h-[48px] bg-slate-800 text-white px-4 rounded-xl font-bold text-sm disabled:opacity-40"
              >
                Terminer filles {girlsComplete ? "✓" : ""}
              </button>
            )}
            <button
              type="button"
              disabled={!canValidate || busy || saveStatus === "saving"}
              onClick={() => void validate()}
              className="flex-1 min-h-[48px] bg-indigo-600 text-white px-4 rounded-xl font-bold text-sm disabled:opacity-40"
            >
              {pendingActivityCount > 0
                ? `Envoi bloqué (${pendingActivityCount} en activité)`
                : "Finaliser & envoyer"}
            </button>
          </div>
          {pendingActivityCount > 0 && boysComplete && girlsComplete && (
            <p className="max-w-3xl mx-auto text-xs text-amber-800 font-semibold text-center sm:text-left">
              Appel terminé pour les autres — attente retour :{" "}
              {pendingActivityNames.slice(0, 4).join(", ")}
              {pendingActivityNames.length > 4 ? "…" : ""}. Puis Présent → envoyer.
            </p>
          )}
          {(boysComplete || girlsComplete) && pendingActivityCount === 0 && (
            <p className="max-w-3xl mx-auto text-xs text-emerald-700 font-semibold text-center sm:text-left">
              {boysComplete && "Garçons OK. "}
              {girlsComplete && "Filles OK."}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
