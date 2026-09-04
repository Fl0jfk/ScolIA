"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  type RhPlanningKind,
  type StaffPlanningDoc,
  type TeacherPlanningCatalog,
  type TeacherPlanningDoc,
  type TeacherPlanningSlot,
  estimateTeacherWeeklyHours,
  findOverlappingTeacherSlots,
} from "@/app/lib/rh/planning-types";
import {
  DayFocusBanner,
  StaffExceptionsPanel,
  StaffQuotaBanner,
  TeacherReplacementsPanel,
} from "@/app/components/personnel/RhPlanningAdvancedSections";
import {
  FixedSlotEditor,
  MissionSlotEditor,
  TeacherSlotEditor,
  TeacherSlotQuickModal,
  WeekGrid,
  applyTeacherSlotQuickEdit,
  detectTeacherSlotWeekMode,
  newId,
  removeTeacherSlotEverywhere,
  type TeacherSlotWeekMode,
} from "@/app/components/personnel/RhPlanningEditors";
import type { PlanningWeekday } from "@/app/lib/rh/planning-types";
import {
  downloadStaffFixedPlanningPdf,
  downloadStaffMissionPlanningPdf,
  downloadTeacherPlanningPdf,
} from "@/app/lib/rh/planning-export-pdf";
import {
  planningSlotCardClass,
  planningSlotMetaTextClass,
  planningSlotTimeClass,
  planningSlotTitleTextClass,
  planningWeekTabClass,
} from "@/app/lib/rh/planning-slot-colors";

type Audience = "teachers" | "staff";

type PersonOption = {
  id: string;
  displayName: string;
  category: string;
  jobTitle?: string | null;
};

function formatPlanningUpdatedAt(iso?: string | null) {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  const absolute = d.toLocaleString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
  const diffMs = Date.now() - d.getTime();
  const days = Math.floor(diffMs / (24 * 60 * 60 * 1000));
  let relative = "à l’instant";
  if (days >= 1) relative = `il y a ${days} j`;
  else {
    const hours = Math.floor(diffMs / (60 * 60 * 1000));
    if (hours >= 1) relative = `il y a ${hours} h`;
    else {
      const mins = Math.max(1, Math.floor(diffMs / (60 * 1000)));
      relative = `il y a ${mins} min`;
    }
  }
  return { absolute, relative };
}


export default function RhPlanningPanel() {
  const [audience, setAudience] = useState<Audience>("teachers");
  const [people, setPeople] = useState<PersonOption[]>([]);
  const [personnelId, setPersonnelId] = useState<string>("");
  const [displayName, setDisplayName] = useState("");
  const [category, setCategory] = useState("");
  const [kind, setKind] = useState<RhPlanningKind>("teacher");
  const [teacher, setTeacher] = useState<TeacherPlanningDoc | null>(null);
  const [staff, setStaff] = useState<StaffPlanningDoc | null>(null);
  const [weekView, setWeekView] = useState<"A" | "B">("A");
  const [rotationId, setRotationId] = useState<string>("");
  const [canEdit, setCanEdit] = useState(false);
  const [canManage, setCanManage] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [editMode, setEditMode] = useState(false);
  const [importing, setImporting] = useState(false);
  const [exportingPdf, setExportingPdf] = useState(false);
  const [importWarnings, setImportWarnings] = useState<string[]>([]);
  const [previewMode, setPreviewMode] = useState(false);
  const [mergeStrategy, setMergeStrategy] = useState<"replace" | "append_rotation">("replace");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [focusDate, setFocusDate] = useState(() => new Date().toISOString().slice(0, 10));
  const [catalog, setCatalog] = useState<TeacherPlanningCatalog | null>(null);
  const [quickSlot, setQuickSlot] = useState<TeacherPlanningSlot | null>(null);
  const [quickSlotPreviousId, setQuickSlotPreviousId] = useState<string | null>(null);
  const [quickWeekMode, setQuickWeekMode] = useState<TeacherSlotWeekMode>("A");
  const [showDetailedList, setShowDetailedList] = useState(false);

  const updatedMeta = formatPlanningUpdatedAt(
    kind === "teacher" ? teacher?.updatedAt : staff?.updatedAt,
  );
  const sourceFileName =
    kind === "teacher" ? teacher?.sourceFileName : staff?.sourceFileName;

  const loadPeople = useCallback(async (aud: Audience) => {
    const res = await fetch(`/api/rh/planning?audience=${aud}`, { cache: "no-store" });
    const j = await res.json();
    if (!res.ok) {
      // Collaborateur sans vue liste : on charge son propre planning.
      setPeople([]);
      setCanManage(false);
      return null;
    }
    setPeople(j.people || []);
    setCanManage(!!j.canManage);
    return (j.people || []) as PersonOption[];
  }, []);

  const loadPlanning = useCallback(async (id?: string, forcedKind?: RhPlanningKind) => {
    setLoading(true);
    setError(null);
    setMsg(null);
    try {
      const params = new URLSearchParams();
      if (id) params.set("personnelId", id);
      if (forcedKind) params.set("kind", forcedKind);
      const q = params.toString() ? `?${params}` : "";
      const res = await fetch(`/api/rh/planning${q}`, { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setPersonnelId(j.personnelId);
      setDisplayName(j.displayName || "");
      setCategory(j.category || "");
      setKind(j.kind);
      setCanEdit(!!j.canEdit);
      setCanManage(!!j.canManage);
      setCatalog((j.catalog as TeacherPlanningCatalog | null) ?? null);
      if (j.kind === "teacher") {
        setTeacher(j.planning as TeacherPlanningDoc);
        setStaff(null);
      } else {
        setStaff(j.planning as StaffPlanningDoc);
        setTeacher(null);
        const firstRot = (j.planning as StaffPlanningDoc).rotations?.[0]?.id;
        setRotationId(firstRot || "");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      const list = await loadPeople(audience);
      if (list && list.length > 0) {
        const preferred = list.find((p) => p.id === personnelId) || list[0]!;
        setPersonnelId(preferred.id);
        await loadPlanning(preferred.id, audience === "teachers" ? "teacher" : "staff");
      } else {
        // Vue collaborateur : charge le planning selon l’audience demandée.
        setLoading(true);
        setError(null);
        try {
          const kind = audience === "teachers" ? "teacher" : "staff";
          const res = await fetch(`/api/rh/planning?kind=${kind}`, { cache: "no-store" });
          const j = await res.json();
          if (!res.ok) throw new Error(j.error || "Chargement impossible");
          setPersonnelId(j.personnelId);
          setDisplayName(j.displayName || "");
          setCategory(j.category || "");
          setKind(j.kind);
          setCanEdit(!!j.canEdit);
          setCanManage(!!j.canManage);
          if (j.kind === "teacher") {
            setTeacher(j.planning as TeacherPlanningDoc);
            setStaff(null);
          } else {
            setStaff(j.planning as StaffPlanningDoc);
            setTeacher(null);
            setRotationId((j.planning as StaffPlanningDoc).rotations?.[0]?.id || "");
          }
        } catch (e) {
          setError(e instanceof Error ? e.message : "Erreur");
        } finally {
          setLoading(false);
        }
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- init / audience switch
  }, [audience]);

  const onSelectPerson = (id: string) => {
    setPersonnelId(id);
    setEditMode(false);
    setPreviewMode(false);
    setImportWarnings([]);
    void loadPlanning(id, audience === "teachers" ? "teacher" : "staff");
  };

  const save = async (override?: TeacherPlanningDoc | StaffPlanningDoc) => {
    if (!personnelId || !canEdit) return;
    const planning = override || (kind === "teacher" ? teacher : staff);
    if (!planning) return;
    if (planning.kind === "teacher") {
      const overlaps = findOverlappingTeacherSlots(planning.weekA).concat(
        findOverlappingTeacherSlots(planning.weekB),
      );
      if (overlaps.length > 0) {
        setError(overlaps[0]!);
        return;
      }
    }
    setSaving(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/rh/planning", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ personnelId, planning }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      if (j.planning?.kind === "teacher") setTeacher(j.planning);
      else setStaff(j.planning);
      const crossConflicts = Array.isArray(j.conflicts) ? j.conflicts : [];
      if (crossConflicts.length > 0) {
        setMsg(
          `Planning enregistré — ${crossConflicts.length} conflit${crossConflicts.length !== 1 ? "s" : ""} inter-profs (voir EDT établissement).`,
        );
      } else {
        setMsg("Planning enregistré.");
      }
      setEditMode(false);
      setPreviewMode(false);
      setImportWarnings([]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const runPdfImport = async (file: File) => {
    if (!personnelId || !canEdit) return;
    setImporting(true);
    setError(null);
    setMsg(null);
    setImportWarnings([]);
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("personnelId", personnelId);
      fd.append("kind", kind);
      fd.append("mergeStrategy", mergeStrategy);
      if (kind === "staff") {
        fd.append("staffMode", staff?.mode || (category === "surveillant" ? "rotation" : "fixed"));
      }
      const res = await fetch("/api/rh/planning/import", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Import impossible");
      if (j.planning?.kind === "teacher") {
        setTeacher(j.planning);
        setStaff(null);
        setKind("teacher");
      } else {
        setStaff(j.planning);
        setTeacher(null);
        setKind("staff");
        setRotationId(j.planning?.rotations?.[0]?.id || "");
      }
      setImportWarnings(Array.isArray(j.warnings) ? j.warnings : []);
      setPreviewMode(true);
      setEditMode(true);
      const classHint = Array.isArray(j.warnings)
        ? (j.warnings as string[]).find((w) => /Classes reconnues/i.test(w))
        : null;
      setMsg(
        [
          j.personHint ? `Prévisualisation (détecté : ${j.personHint}).` : "Prévisualisation prête.",
          classHint || null,
          "Vérifiez les créneaux puis validez pour enregistrer — les classes nourrissent automatiquement les dossiers élèves.",
        ]
          .filter(Boolean)
          .join(" "),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur import");
    } finally {
      setImporting(false);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  const discardPreview = () => {
    setPreviewMode(false);
    setImportWarnings([]);
    setEditMode(false);
    void loadPlanning(personnelId, audience === "teachers" ? "teacher" : "staff");
  };

  const teacherSlots = weekView === "A" ? teacher?.weekA || [] : teacher?.weekB || [];

  const openQuickEdit = (slotId: string) => {
    if (!teacher) return;
    const slot =
      teacherSlots.find((s) => s.id === slotId) ||
      teacher.weekA.find((s) => s.id === slotId) ||
      teacher.weekB.find((s) => s.id === slotId);
    if (!slot) return;
    setQuickSlot({ ...slot, classes: [...(slot.classes || [])] });
    setQuickSlotPreviousId(slot.id);
    setQuickWeekMode(detectTeacherSlotWeekMode(teacher, slot, weekView));
    setEditMode(true);
  };

  const openQuickCreate = (day: PlanningWeekday, start: string, end: string) => {
    setQuickSlot({
      id: newId("slot"),
      day,
      start,
      end,
      subject: "",
      classes: [],
      room: "",
    });
    setQuickSlotPreviousId(null);
    setQuickWeekMode(weekView === "B" ? "B" : "A");
    setEditMode(true);
  };

  const applyQuickSlot = () => {
    if (!teacher || !quickSlot) return;
    if (!quickSlot.subject.trim()) {
      setError("Indiquez une matière pour le créneau.");
      return;
    }
    if (quickSlot.start >= quickSlot.end) {
      setError("L’heure de fin doit être après l’heure de début.");
      return;
    }
    setError(null);
    setTeacher(applyTeacherSlotQuickEdit(teacher, quickSlot, quickWeekMode, quickSlotPreviousId));
    if (quickWeekMode === "A" || quickWeekMode === "both") setWeekView("A");
    else setWeekView("B");
    setQuickSlot(null);
    setQuickSlotPreviousId(null);
    setMsg("Créneau mis à jour sur la grille — pensez à enregistrer.");
  };

  const deleteQuickSlot = () => {
    if (!teacher || !quickSlotPreviousId) {
      setQuickSlot(null);
      return;
    }
    setTeacher(removeTeacherSlotEverywhere(teacher, quickSlotPreviousId));
    setQuickSlot(null);
    setQuickSlotPreviousId(null);
    setMsg("Créneau supprimé — pensez à enregistrer.");
  };

  const activeRotation = useMemo(() => {
    if (!staff) return null;
    return staff.rotations.find((r) => r.id === rotationId) || staff.rotations[0] || null;
  }, [staff, rotationId]);

  const teacherHours = useMemo(
    () => (teacher ? estimateTeacherWeeklyHours(teacher) : null),
    [teacher],
  );

  const focusException = useMemo(() => {
    if (!staff || staff.mode !== "fixed") return null;
    return (staff.exceptions || []).find((e) => e.date === focusDate) || null;
  }, [staff, focusDate]);

  const focusReplacements = useMemo(() => {
    if (!teacher) return [];
    return (teacher.replacements || []).filter((r) => r.date === focusDate);
  }, [teacher, focusDate]);

  const exportPdf = async () => {
    setExportingPdf(true);
    setError(null);
    try {
      const name = displayName || "Planning";
      if (kind === "teacher" && teacher) {
        const slots = weekView === "A" ? teacher.weekA : teacher.weekB;
        await downloadTeacherPlanningPdf({ displayName: name, week: weekView, slots });
      } else if (kind === "staff" && staff) {
        if (staff.mode === "fixed") {
          await downloadStaffFixedPlanningPdf({ displayName: name, slots: staff.fixedSlots });
        } else if (activeRotation) {
          await downloadStaffMissionPlanningPdf({
            displayName: name,
            rotationLabel: activeRotation.label,
            slots: activeRotation.slots,
          });
        }
      }
      setMsg("PDF téléchargé.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Export PDF impossible");
    } finally {
      setExportingPdf(false);
    }
  };

  const hasExportableSlots =
    (kind === "teacher" && teacher && teacherSlots.length > 0) ||
    (kind === "staff" &&
      staff &&
      (staff.mode === "fixed" ? staff.fixedSlots.length > 0 : (activeRotation?.slots.length ?? 0) > 0));

  return (
    <div className="space-y-5">
      <div className="bg-white rounded-2xl border border-slate-100 p-5 shadow-sm space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
          <div>
            <h2 className="font-black text-slate-800 text-lg">Planning RH</h2>
            <p className="text-sm text-slate-500 mt-0.5">
              Profs : semaines types A/B (année). OGEC : semaine type + quota, ou missions / lieux
              (surveillants).
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            {(
              [
                ["teachers", "Professeurs"],
                ["staff", "Personnels OGEC"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => {
                  setAudience(id);
                  setEditMode(false);
                  setPreviewMode(false);
                  setImportWarnings([]);
                }}
                className={`px-3 py-1.5 rounded-xl text-xs font-bold ${
                  audience === id
                    ? "bg-indigo-600 text-white"
                    : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        {canManage || people.length > 0 ? (
          <label className="block text-sm">
            <span className="text-xs font-bold text-slate-500 uppercase tracking-wide">Collaborateur</span>
            <select
              className="mt-1 w-full sm:max-w-md border border-slate-200 rounded-xl px-3 py-2 text-sm font-medium"
              value={personnelId}
              onChange={(e) => onSelectPerson(e.target.value)}
            >
              {people.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.displayName}
                  {p.jobTitle ? ` — ${p.jobTitle}` : ""}
                </option>
              ))}
            </select>
          </label>
        ) : displayName ? (
          <p className="text-sm font-semibold text-slate-700">
            Mon planning — {displayName}
            {category ? (
              <span className="text-slate-400 font-medium"> ({category})</span>
            ) : null}
          </p>
        ) : null}

        {updatedMeta ? (
          <div className="rounded-xl border border-emerald-100 bg-emerald-50/80 px-3 py-2 text-sm text-emerald-900">
            <p className="font-bold">
              Dernière mise à jour : {updatedMeta.absolute}
              <span className="font-medium text-emerald-700/80"> ({updatedMeta.relative})</span>
            </p>
            {sourceFileName ? (
              <p className="text-xs text-emerald-800/80 mt-0.5">Source PDF : {sourceFileName}</p>
            ) : null}
          </div>
        ) : !loading ? (
          <p className="text-xs text-slate-400 italic">Aucun planning enregistré pour l’instant.</p>
        ) : null}

        {!loading && kind === "teacher" && teacher && teacherHours ? (
          <p className="text-xs font-semibold text-indigo-800 bg-indigo-50 border border-indigo-100 rounded-xl px-3 py-2">
            Semaines types A/B — valables toute l’année scolaire. Volume : A {teacherHours.weekA} h
            · B {teacherHours.weekB} h · moy. {teacherHours.averageWeekly} h. Les remplacements sont
            des créneaux datés en plus.
          </p>
        ) : null}
        {!loading && kind === "staff" && staff?.mode === "fixed" ? (
          <p className="text-xs font-semibold text-amber-900 bg-amber-50 border border-amber-100 rounded-xl px-3 py-2">
            Semaine type annuelle (admin, compta, CPE…) — exceptions jour / avance sur le quota
            possible. Pas le même modèle que la surveillance (missions liées aux élèves).
          </p>
        ) : null}
        {!loading && kind === "staff" && staff?.mode === "rotation" ? (
          <p className="text-xs font-semibold text-emerald-900 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
            Missions & lieux (entrée, cour, étude…) pour l’éducation / surveillance — engagés sur
            le temps de présence des élèves ; rechargeable souvent via PDF.
          </p>
        ) : null}

        {canEdit && personnelId ? (
          <div className="rounded-xl border border-dashed border-indigo-200 bg-indigo-50/40 p-3 space-y-2">
            <div className="flex flex-wrap items-center gap-2">
              <p className="text-xs font-bold text-indigo-900 flex-1">
                Importer un PDF (OCR + IA) — remplace ou met à jour le planning après validation
              </p>
              <input
                ref={fileInputRef}
                type="file"
                accept="application/pdf,.pdf"
                className="hidden"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) void runPdfImport(f);
                }}
              />
              <button
                type="button"
                disabled={importing || !personnelId}
                onClick={() => fileInputRef.current?.click()}
                className="px-3 py-1.5 rounded-xl text-xs font-bold bg-indigo-600 text-white hover:bg-indigo-700 disabled:opacity-60"
              >
                {importing ? "Analyse IA…" : previewMode ? "Recharger un PDF" : "Charger un PDF"}
              </button>
            </div>
            {kind === "staff" ? (
              <label className="flex flex-wrap items-center gap-2 text-xs text-slate-600">
                <span className="font-bold">Si missions / surveillants :</span>
                <select
                  className="border border-slate-200 rounded-lg px-2 py-1 text-xs font-semibold bg-white"
                  value={mergeStrategy}
                  onChange={(e) =>
                    setMergeStrategy(e.target.value === "append_rotation" ? "append_rotation" : "replace")
                  }
                >
                  <option value="replace">Remplacer le planning</option>
                  <option value="append_rotation">Ajouter une nouvelle variante (historique)</option>
                </select>
              </label>
            ) : (
              <p className="text-[11px] text-indigo-800/80">
                Semaines types A/B pour toute l’année — détectées automatiquement si le PDF les
                distingue.
              </p>
            )}
          </div>
        ) : null}

        {previewMode ? (
          <div className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 space-y-2">
            <p className="text-sm font-bold text-amber-900">
              Prévisualisation — pas encore enregistré. Vérifiez la grille puis validez.
            </p>
            {importWarnings.length > 0 ? (
              <ul className="text-xs text-amber-800 list-disc pl-4 space-y-0.5">
                {importWarnings.map((w) => (
                  <li key={w}>{w}</li>
                ))}
              </ul>
            ) : null}
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={saving}
                onClick={() => void save()}
                className="px-3 py-1.5 rounded-xl text-xs font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
              >
                {saving ? "Validation…" : "Valider et enregistrer"}
              </button>
              <button
                type="button"
                onClick={discardPreview}
                className="px-3 py-1.5 rounded-xl text-xs font-bold border border-amber-300 text-amber-900 bg-white"
              >
                Annuler l’import
              </button>
            </div>
          </div>
        ) : null}

        {error ? <p className="text-sm text-rose-600">{error}</p> : null}
        {msg ? <p className="text-sm text-emerald-700">{msg}</p> : null}

        {loading ? (
          <p className="text-sm text-slate-400">Chargement…</p>
        ) : (
          <>
            <div className="flex flex-wrap items-center gap-2">
              {kind === "teacher" ? (
                <>
                  {(
                    [
                      ["A", "Semaine type A"],
                      ["B", "Semaine type B"],
                    ] as const
                  ).map(([id, label]) => (
                    <button
                      key={id}
                      type="button"
                      onClick={() => setWeekView(id)}
                      className={`px-3 py-1.5 rounded-lg text-xs font-bold ${planningWeekTabClass(weekView === id, id)}`}
                    >
                      {label}
                    </button>
                  ))}
                  {hasExportableSlots ? (
                    <button
                      type="button"
                      disabled={exportingPdf}
                      onClick={() => void exportPdf()}
                      className="px-3 py-1.5 rounded-lg text-xs font-bold border border-slate-200 bg-white text-slate-700 hover:bg-slate-50 disabled:opacity-60"
                    >
                      {exportingPdf ? "Export…" : "Exporter PDF"}
                    </button>
                  ) : null}
                </>
              ) : staff?.mode === "rotation" && staff.rotations.length > 0 ? (
                <>
                  <select
                    className="border border-slate-200 rounded-lg px-2 py-1.5 text-xs font-bold"
                    value={activeRotation?.id || ""}
                    onChange={(e) => setRotationId(e.target.value)}
                  >
                    {staff.rotations.map((r) => (
                      <option key={r.id} value={r.id}>
                        {r.label}
                      </option>
                    ))}
                  </select>
                  {hasExportableSlots ? (
                    <button
                      type="button"
                      disabled={exportingPdf}
                      onClick={() => void exportPdf()}
                      className="px-3 py-1.5 rounded-lg text-xs font-bold border border-slate-200 bg-white text-slate-700 hover:bg-slate-50 disabled:opacity-60"
                    >
                      {exportingPdf ? "Export…" : "Exporter PDF"}
                    </button>
                  ) : null}
                </>
              ) : (
                <>
                  <span className="text-xs font-bold text-slate-500 uppercase tracking-wide">
                    Semaine type annuelle
                  </span>
                  {hasExportableSlots ? (
                    <button
                      type="button"
                      disabled={exportingPdf}
                      onClick={() => void exportPdf()}
                      className="px-3 py-1.5 rounded-lg text-xs font-bold border border-slate-200 bg-white text-slate-700 hover:bg-slate-50 disabled:opacity-60"
                    >
                      {exportingPdf ? "Export…" : "Exporter PDF"}
                    </button>
                  ) : null}
                </>
              )}

              {canEdit ? (
                <div className="ml-auto flex flex-wrap gap-2">
                  {!editMode ? (
                    <button
                      type="button"
                      onClick={() => setEditMode(true)}
                      className="px-3 py-1.5 rounded-xl text-xs font-bold bg-indigo-600 text-white hover:bg-indigo-700"
                    >
                      Éditer mon horaire type
                    </button>
                  ) : (
                    <>
                      <button
                        type="button"
                        onClick={() => {
                          setEditMode(false);
                          void loadPlanning(
                            personnelId,
                            audience === "teachers" ? "teacher" : "staff",
                          );
                        }}
                        className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 text-slate-600"
                      >
                        Annuler
                      </button>
                      <button
                        type="button"
                        disabled={saving}
                        onClick={() => void save()}
                        className="px-3 py-1.5 rounded-xl text-xs font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
                      >
                        {saving ? "Enregistrement…" : "Enregistrer"}
                      </button>
                    </>
                  )}
                </div>
              ) : null}
            </div>

            {!loading ? (
              <DayFocusBanner
                date={focusDate}
                onDateChange={setFocusDate}
                exception={focusException}
                replacements={focusReplacements}
              />
            ) : null}

            {kind === "teacher" && teacher ? (
              <>
                {canEdit ? (
                  <p className="text-xs text-slate-600 rounded-xl border border-indigo-100 bg-indigo-50/70 px-3 py-2">
                    <span className="font-bold text-indigo-900">Édition visuelle :</span> cliquez un
                    créneau pour changer classe / horaires / semaine A ou B. Cliquez dans une zone
                    vide du calendrier pour ajouter un créneau
                    {editMode || previewMode ? "" : " (passe en mode édition automatiquement)"}.
                  </p>
                ) : null}
                <WeekGrid
                  slots={teacherSlots}
                  editable={Boolean(canEdit)}
                  selectedSlotId={quickSlotPreviousId}
                  onSlotClick={
                    canEdit
                      ? (slotId) => {
                          if (!editMode && !previewMode) setEditMode(true);
                          openQuickEdit(slotId);
                        }
                      : undefined
                  }
                  onEmptyClick={
                    canEdit
                      ? (day, start, end) => {
                          if (!editMode && !previewMode) setEditMode(true);
                          openQuickCreate(day, start, end);
                        }
                      : undefined
                  }
                  renderCard={(slot) => {
                    const full = teacherSlots.find((s) => s.id === slot.id) as TeacherPlanningSlot;
                    const colorKey = full.subject || "cours";
                    return (
                      <div className={planningSlotCardClass(colorKey)}>
                        <p className={planningSlotTimeClass(colorKey)}>
                          {full.start}–{full.end}
                        </p>
                        <p className={planningSlotTitleTextClass(colorKey)}>{full.subject || "—"}</p>
                        <p className={planningSlotMetaTextClass(colorKey)}>
                          {(full.classes || []).join(", ") || "Classe ?"}
                          {full.room ? ` · ${full.room}` : ""}
                        </p>
                      </div>
                    );
                  }}
                />
                {editMode ? (
                  <div className="space-y-2">
                    <button
                      type="button"
                      onClick={() => setShowDetailedList((v) => !v)}
                      className="text-xs font-bold text-slate-500 hover:text-indigo-700"
                    >
                      {showDetailedList
                        ? "Masquer la liste détaillée"
                        : "Afficher la liste détaillée (tous les champs)"}
                    </button>
                    {showDetailedList ? (
                      <TeacherSlotEditor
                        slots={teacherSlots}
                        catalog={catalog}
                        onChange={(slots) => {
                          setTeacher((prev) =>
                            prev
                              ? weekView === "A"
                                ? { ...prev, weekA: slots }
                                : { ...prev, weekB: slots }
                              : prev,
                          );
                        }}
                      />
                    ) : null}
                  </div>
                ) : null}
                <TeacherReplacementsPanel
                  teacher={teacher}
                  canManage={canManage}
                  onChange={(replacements) => {
                    setTeacher((prev) => (prev ? { ...prev, replacements } : prev));
                    if (canManage) setEditMode(true);
                  }}
                />
                {canManage && (teacher.replacements?.length ?? 0) > 0 && !previewMode ? (
                  <button
                    type="button"
                    disabled={saving}
                    onClick={() => void save()}
                    className="px-3 py-1.5 rounded-xl text-xs font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
                  >
                    {saving ? "Enregistrement…" : "Enregistrer remplacements"}
                  </button>
                ) : null}
              </>
            ) : null}

            {kind === "staff" && staff ? (
              <>
                {staff.mode === "fixed" ? (
                  <StaffQuotaBanner
                    staff={staff}
                    canEdit={canEdit}
                    onChangeTarget={(annualHoursTarget) => {
                      setStaff((prev) =>
                        prev ? { ...prev, annualHoursTarget } : prev,
                      );
                      setEditMode(true);
                    }}
                  />
                ) : null}
                {canManage && editMode ? (
                  <div className="flex flex-wrap gap-2 items-center">
                    <span className="text-xs font-bold text-slate-500">Mode</span>
                    {(
                      [
                        ["fixed", "Planning fixe"],
                        ["rotation", "Missions / rotation"],
                      ] as const
                    ).map(([id, label]) => (
                      <button
                        key={id}
                        type="button"
                        onClick={() =>
                          setStaff((prev) =>
                            prev
                              ? {
                                  ...prev,
                                  mode: id,
                                  rotations:
                                    id === "rotation" && prev.rotations.length === 0
                                      ? [
                                          {
                                            id: newId("rot"),
                                            label: "Semaine type",
                                            startDate: null,
                                            endDate: null,
                                            slots: [],
                                          },
                                        ]
                                      : prev.rotations,
                                }
                              : prev,
                          )
                        }
                        className={`px-3 py-1.5 rounded-lg text-xs font-bold ${
                          staff.mode === id
                            ? "bg-amber-600 text-white"
                            : "bg-white border border-slate-200 text-slate-600"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                ) : null}

                {staff.mode === "fixed" ? (
                  <>
                    <WeekGrid
                      slots={staff.fixedSlots}
                      renderCard={(slot) => {
                        const full = staff.fixedSlots.find((s) => s.id === slot.id)!;
                        const colorKey = full.label || "poste";
                        return (
                          <div className={planningSlotCardClass(colorKey)}>
                            <p className={planningSlotTimeClass(colorKey)}>
                              {full.start}–{full.end}
                            </p>
                            <p className={planningSlotTitleTextClass(colorKey)}>{full.label || "—"}</p>
                          </div>
                        );
                      }}
                    />
                    {editMode ? (
                      <FixedSlotEditor
                        slots={staff.fixedSlots}
                        onChange={(fixedSlots) => setStaff((p) => (p ? { ...p, fixedSlots } : p))}
                      />
                    ) : null}
                    <StaffExceptionsPanel
                      staff={staff}
                      canEdit={canEdit}
                      onChange={(exceptions) => {
                        setStaff((prev) => (prev ? { ...prev, exceptions } : prev));
                        setEditMode(true);
                      }}
                    />
                    {canEdit && (staff.exceptions?.length || staff.annualHoursTarget != null) && !previewMode ? (
                      <button
                        type="button"
                        disabled={saving}
                        onClick={() => void save()}
                        className="px-3 py-1.5 rounded-xl text-xs font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
                      >
                        {saving ? "Enregistrement…" : "Enregistrer quota / exceptions"}
                      </button>
                    ) : null}
                  </>
                ) : (
                  <>
                    <WeekGrid
                      slots={activeRotation?.slots || []}
                      renderCard={(slot) => {
                        const full = (activeRotation?.slots || []).find((s) => s.id === slot.id)!;
                        const colorKey = full.mission || "mission";
                        return (
                          <div className={planningSlotCardClass(colorKey)}>
                            <p className={planningSlotTimeClass(colorKey)}>
                              {full.start}–{full.end}
                            </p>
                            <p className={planningSlotTitleTextClass(colorKey)}>{full.mission || "—"}</p>
                            <p className={planningSlotMetaTextClass(colorKey)}>
                              {full.location || "Lieu non précisé"}
                            </p>
                          </div>
                        );
                      }}
                    />
                    {editMode && activeRotation ? (
                      <MissionSlotEditor
                        slots={activeRotation.slots}
                        onChange={(slots) =>
                          setStaff((prev) => {
                            if (!prev) return prev;
                            return {
                              ...prev,
                              rotations: prev.rotations.map((r) =>
                                r.id === activeRotation.id ? { ...r, slots } : r,
                              ),
                            };
                          })
                        }
                        onAddRotation={() => {
                          const id = newId("rot");
                          setStaff((prev) =>
                            prev
                              ? {
                                  ...prev,
                                  rotations: [
                                    ...prev.rotations,
                                    {
                                      id,
                                      label: `Variante ${prev.rotations.length + 1}`,
                                      startDate: null,
                                      endDate: null,
                                      slots: [],
                                    },
                                  ],
                                }
                              : prev,
                          );
                          setRotationId(id);
                        }}
                        rotationLabel={activeRotation.label}
                        onRenameRotation={(label) =>
                          setStaff((prev) =>
                            prev
                              ? {
                                  ...prev,
                                  rotations: prev.rotations.map((r) =>
                                    r.id === activeRotation.id ? { ...r, label } : r,
                                  ),
                                }
                              : prev,
                          )
                        }
                      />
                    ) : null}
                  </>
                )}
              </>
            ) : null}
          </>
        )}
      </div>

      {quickSlot ? (
        <TeacherSlotQuickModal
          open
          title={quickSlotPreviousId ? "Modifier le créneau" : "Nouveau créneau"}
          slot={quickSlot}
          weekMode={quickWeekMode}
          catalog={catalog}
          onWeekModeChange={setQuickWeekMode}
          onChange={setQuickSlot}
          onClose={() => {
            setQuickSlot(null);
            setQuickSlotPreviousId(null);
          }}
          onSave={applyQuickSlot}
          onDelete={quickSlotPreviousId ? deleteQuickSlot : undefined}
        />
      ) : null}
    </div>
  );
}

