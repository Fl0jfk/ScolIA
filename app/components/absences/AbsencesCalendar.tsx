"use client";

import { useUser } from "@clerk/nextjs";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  canManageAbsence,
  resolveAbsenceScope,
} from "@/app/lib/absences-types";
import type { AbsenceRecord, AbsenceScope, Etablissement } from "@/app/lib/absences-types";
import type { AbsencePeriodType } from "@/app/lib/absence-period";
import {
  absencesToCalendarEvents,
  appearanceForEvent,
  buildTeacherColorIndexMap,
  dedupeCalendarEventsForDisplay,
  type CalendarEvent,
} from "@/app/lib/absences-calendar";
import { PaperclipIcon, PrinterIcon, printDaySummary } from "@/app/components/absences/AbsencesCalendarPrint";

const DAYS = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];

function sameDay(date: Date, y: number, m: number, d: number) {
  return date.getFullYear() === y && date.getMonth() === m && date.getDate() === d;
}

function isTodayDate(date: Date) {
  const now = new Date();
  return sameDay(date, now.getFullYear(), now.getMonth(), now.getDate());
}

function calendarDayCellClass(date: Date | null, variant: "desktop" | "mobile") {
  const base =
    variant === "mobile" ? "min-h-[108px] border rounded-xl p-2" : "min-h-[120px] h-full border rounded-xl p-2";
  if (!date) return `${base} border-transparent bg-transparent`;
  if (isTodayDate(date)) return `${base} border-sky-400 bg-sky-50/90 ring-2 ring-sky-200/70`;
  return `${base} border-slate-100 bg-slate-50/40`;
}

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

function isoToTime(iso: string) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

function recordToEditForm(record: AbsenceRecord) {
  const d = record.data;
  const periodType: AbsencePeriodType =
    d.periodType ?? (d.startTime && d.endTime ? "single_day" : "multi_day");
  const scope = resolveAbsenceScope(record);
  return {
    displayName: record.displayName,
    reason: d.reason,
    scope,
    etablissement: (d.etablissement || "Collège") as Etablissement,
    periodType,
    startDate: d.startDate || d.startAt.slice(0, 10),
    endDate: d.endDate || d.endAt.slice(0, 10),
    startTime: (d.startTime || isoToTime(d.startAt) || "08:00").slice(0, 5),
    endTime: (d.endTime || isoToTime(d.endAt) || "18:00").slice(0, 5),
  };
}

type EditFormState = ReturnType<typeof recordToEditForm>;


type AbsencesCalendarProps = {
  refreshKey?: number;
};

export default function AbsencesCalendar({ refreshKey = 0 }: AbsencesCalendarProps) {
  const { user, isLoaded: userLoaded } = useUser();
  const [items, setItems] = useState<AbsenceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [attachingPdf, setAttachingPdf] = useState(false);
  const attachPdfRef = useRef<HTMLInputElement>(null);
  const attachTargetIdRef = useRef<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [currentMonth, setCurrentMonth] = useState(new Date());
  const selectedYear = currentMonth.getFullYear();
  const [editRecord, setEditRecord] = useState<AbsenceRecord | null>(null);
  const [editForm, setEditForm] = useState<EditFormState | null>(null);
  const [editSaving, setEditSaving] = useState(false);
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const media = window.matchMedia("(max-width: 639px)");
    const update = () => setIsMobile(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  const roles = useMemo(() => (userLoaded ? rolesFromUserLike(user) : []), [user, userLoaded]);

  const canViewDocumentsFor = useMemo(() => {
    // Pièces jointes absences : jamais affichées (trop sensibles).
    return (_item?: AbsenceRecord) => false;
  }, []);

  const canManageDocsFor = useMemo(() => {
    return (_item?: AbsenceRecord) => false;
  }, []);

  const canManageSlot = useMemo(() => {
    return (item: AbsenceRecord) => canManageAbsence(item, roles);
  }, [roles]);

  const fetchAbsences = async () => {
    try {
      setLoading(true);
      const res = await fetch("/api/absences?calendar=true", { cache: "no-store" });
      if (!res.ok) throw new Error("Chargement des absences impossible.");
      const data = (await res.json()) as AbsenceRecord[];
      setItems(data || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur de chargement.");
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    fetchAbsences();
  }, [refreshKey]);

  const teacherColorIndexMap = useMemo(
    () =>
      buildTeacherColorIndexMap(
        items.filter((item) => item.data.scope === "professeur").map((item) => item.displayName),
      ),
    [items],
  );

  const events = useMemo<CalendarEvent[]>(
    () => absencesToCalendarEvents(items, { includeDocumentsFor: canViewDocumentsFor }),
    [items, canViewDocumentsFor],
  );

  const itemsById = useMemo(() => new Map(items.map((item) => [item.id, item])), [items]);

  const openConvocation = async (absenceId: string, docIndex = 0) => {
    setError(null);
    try {
      const res = await fetch(
        `/api/absences/document-url?id=${encodeURIComponent(absenceId)}&index=${encodeURIComponent(String(docIndex))}`,
        { cache: "no-store" },
      );
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Impossible d'ouvrir le document.");
      const url = String(payload?.url || "");
      if (!url) throw new Error("URL du document manquante.");
      window.open(url, "_blank", "noopener,noreferrer");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur ouverture document.");
    }
  };

  const triggerAttachPdf = (absenceId: string) => {
    attachTargetIdRef.current = absenceId;
    attachPdfRef.current?.click();
  };

  const handleAttachPdf = async (file: File) => {
    const absenceId = attachTargetIdRef.current;
    if (!absenceId) return;
    setError(null);
    setSuccess(null);
    const fd = new FormData();
    fd.append("id", absenceId);
    fd.append("file", file);
    try {
      setAttachingPdf(true);
      const res = await fetch("/api/absences/attach-document", { method: "POST", body: fd });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Ajout du PDF impossible.");
      setSuccess(`PDF ajouté (${payload?.documentCount || 1} document(s) sur ce créneau).`);
      await fetchAbsences();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur lors de l'ajout du PDF.");
    } finally {
      setAttachingPdf(false);
      attachTargetIdRef.current = null;
    }
  };

  const deleteDocument = async (absenceId: string, docIndex = 0) => {
    const item = itemsById.get(absenceId);
    if (!item || !canManageDocsFor(item)) return;
    const ok = window.confirm("Supprimer ce document ?");
    if (!ok) return;
    setError(null);
    setSuccess(null);
    try {
      const res = await fetch("/api/absences/delete-document", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: absenceId, index: docIndex }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Suppression impossible.");
      setSuccess("Document supprimé.");
      await fetchAbsences();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur suppression document.");
    }
  };

  const openEdit = (id: string) => {
    const item = itemsById.get(id);
    if (!item || !canManageSlot(item)) return;
    setEditRecord(item);
    setEditForm(recordToEditForm(item));
  };

  const saveEdit = async () => {
    if (!editRecord || !editForm) return;
    setEditSaving(true);
    setError(null);
    setSuccess(null);
    try {
      const res = await fetch("/api/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id: editRecord.id,
          action: "MODIFIER_CALENDRIER",
          ...editForm,
          endDate: editForm.periodType === "single_day" ? editForm.startDate : editForm.endDate,
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Modification impossible.");
      setSuccess("Créneau modifié.");
      setEditRecord(null);
      setEditForm(null);
      await fetchAbsences();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur modification.");
    } finally {
      setEditSaving(false);
    }
  };

  const deleteConvocation = async (id: string) => {
    const item = itemsById.get(id);
    if (!item || !canManageSlot(item)) return;
    setError(null);
    setSuccess(null);
    const ok = window.confirm("Supprimer ce créneau ? Cela supprime aussi le document s'il n'est plus utilisé.");
    if (!ok) return;
    try {
      const res = await fetch(`/api/absences?id=${encodeURIComponent(id)}`, { method: "DELETE" });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Suppression impossible.");
      setSuccess("Créneau supprimé.");
      await fetchAbsences();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur suppression.");
    }
  };

  const dayCells = useMemo(() => {
    const year = currentMonth.getFullYear();
    const month = currentMonth.getMonth();
    const firstDay = new Date(year, month, 1);
    const firstDayOffset = (firstDay.getDay() + 6) % 7;
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const cells: Array<{ key: string; date: Date | null; events: CalendarEvent[] }> = [];

    for (let i = 0; i < firstDayOffset; i += 1) {
      cells.push({ key: `empty-start-${i}`, date: null, events: [] });
    }

    for (let day = 1; day <= daysInMonth; day += 1) {
      const date = new Date(year, month, day);
      const dayEvents = dedupeCalendarEventsForDisplay(
        events.filter((event) => {
          const eventDate = new Date(event.startAt);
          return sameDay(eventDate, year, month, day);
        }),
      );
      cells.push({ key: `d-${day}`, date, events: dayEvents });
    }

    while (cells.length % 7 !== 0) {
      cells.push({ key: `empty-end-${cells.length}`, date: null, events: [] });
    }
    return cells;
  }, [currentMonth, events]);

  const mobileTodayCell = useMemo(() => {
    const now = new Date();
    const year = now.getFullYear();
    const month = now.getMonth();
    const day = now.getDate();
    const date = new Date(year, month, day);
    const dayEvents = dedupeCalendarEventsForDisplay(
      events.filter((event) => {
        const eventDate = new Date(event.startAt);
        return sameDay(eventDate, year, month, day);
      }),
    );
    return { key: "mobile-today", date, events: dayEvents };
  }, [events]);
  const yearOptions = useMemo(() => {
    const years = new Set<number>();
    years.add(new Date().getFullYear());
    years.add(selectedYear);
    for (const item of items) {
      const y1 = new Date(item.data.startAt).getFullYear();
      const y2 = new Date(item.data.endAt).getFullYear();
      if (!Number.isNaN(y1)) years.add(y1);
      if (!Number.isNaN(y2)) years.add(y2);
    }
    const list = [...years].sort((a, b) => a - b);
    const min = list[0] ?? selectedYear;
    const max = list[list.length - 1] ?? selectedYear;
    const padded: number[] = [];
    for (let y = min - 2; y <= max + 2; y += 1) padded.push(y);
    return padded;
  }, [items, selectedYear]);

  return (
    <div className="max-w-7xl mx-auto p-6 space-y-6">
      <input
        ref={attachPdfRef}
        type="file"
        accept="application/pdf"
        className="hidden"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) handleAttachPdf(file);
          e.currentTarget.value = "";
        }}
        disabled={attachingPdf}
      />
      <section className="bg-white border border-slate-200 rounded-none sm:rounded-3xl p-0 sm:p-5 -mx-6 sm:mx-0">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between mb-4 px-4 pt-4 sm:px-0 sm:pt-0">
          <div>
            <h2 className="text-xl font-black text-slate-900">Calendrier des absences</h2>
            {isMobile ? (
            <p className="mt-1 text-sm font-semibold text-slate-500">
              Aujourd&apos;hui —{" "}
              {mobileTodayCell.date.toLocaleDateString("fr-FR", {
                weekday: "long",
                day: "numeric",
                month: "long",
              })}
            </p>
            ) : null}
          </div>
          {!isMobile ? (
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-2">
            <select
              className="w-full sm:w-auto px-3 py-2 sm:py-1 rounded-lg border border-slate-200 bg-white text-sm font-bold text-slate-700"
              value={selectedYear}
              onChange={(e) => {
                const y = Number(e.target.value);
                if (!Number.isFinite(y)) return;
                setCurrentMonth(new Date(y, currentMonth.getMonth(), 1));
              }}
            >
              {yearOptions.map((y) => (
                <option key={y} value={y}>
                  {y}
                </option>
              ))}
            </select>

            <div className="flex items-center justify-between sm:justify-start gap-2">
              <button
                type="button"
                onClick={() => setCurrentMonth(new Date(currentMonth.getFullYear(), currentMonth.getMonth() - 1, 1))}
                className="px-3 py-2 sm:py-1 rounded-lg border border-slate-200 hover:bg-slate-50"
              >
                ◀
              </button>
              <span className="text-sm font-bold text-slate-700 min-w-[140px] text-center flex-1 sm:flex-none">
                {currentMonth.toLocaleDateString("fr-FR", { month: "long", year: "numeric" })}
              </span>
              <button
                type="button"
                onClick={() => setCurrentMonth(new Date(currentMonth.getFullYear(), currentMonth.getMonth() + 1, 1))}
                className="px-3 py-2 sm:py-1 rounded-lg border border-slate-200 hover:bg-slate-50"
              >
                ▶
              </button>
            </div>
          </div>
          ) : null}
        </div>
        {isMobile ? (
        <div className="px-2 pb-2">
          <div className="grid grid-cols-1 gap-2">
            <div key={mobileTodayCell.key} className={calendarDayCellClass(mobileTodayCell.date, "mobile")}>
              {mobileTodayCell.date ? (
                <>
                  <div className="flex items-center gap-1 mb-2">
                    <span className="text-xs font-black text-sky-800">{mobileTodayCell.date.getDate()}</span>
                    <span className="text-[8px] font-black uppercase tracking-wide text-sky-700">Auj.</span>
                  </div>
                  <div className="space-y-1">
                    {mobileTodayCell.events.length === 0 ? (
                      <p className="text-xs font-semibold text-slate-400">Aucune absence aujourd&apos;hui.</p>
                    ) : null}
                    {mobileTodayCell.events.map((event) => (
                        (() => {
                          const appearance = appearanceForEvent(event, teacherColorIndexMap);
                          return (
                        <div
                          key={event.key}
                          role={event.hasDocument ? "button" : undefined}
                          tabIndex={event.hasDocument ? 0 : -1}
                          onClick={() => (event.hasDocument ? openConvocation(event.id) : undefined)}
                          onKeyDown={(e) => {
                            if (!event.hasDocument) return;
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              openConvocation(event.id);
                            }
                          }}
                          title={
                            event.hasDocument
                              ? event.isOgec
                                ? "Ouvrir le justificatif (PDF)"
                                : "Ouvrir la convocation (PDF)"
                              : event.isOgec
                                ? "Absence sans justificatif"
                                : "Convocation sans document"
                          }
                          style={appearance.cardStyle}
                          className={[
                            "relative text-left w-full rounded-lg border px-2 py-2 text-[11px] leading-snug",
                            event.hasDocument
                              ? "transition-[filter] cursor-pointer hover:brightness-[0.97]"
                              : "opacity-80 cursor-default",
                            event.hasDocument ? "focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-slate-300" : "",
                          ].join(" ")}
                        >
                          {itemsById.get(event.id) && canManageSlot(itemsById.get(event.id)!) ? (
                            <div className="flex justify-end gap-0.5 mb-0.5">
                              <button
                                type="button"
                                title="Modifier"
                                className="text-[9px] font-black opacity-80"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  openEdit(event.id);
                                }}
                              >
                                ✎
                              </button>
                              <button
                                type="button"
                                title="Supprimer"
                                className="text-[9px] font-black text-rose-700 opacity-80"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  deleteConvocation(event.id);
                                }}
                              >
                                ✕
                              </button>
                            </div>
                          ) : null}
                          <div className="text-center text-sm font-black leading-snug break-words whitespace-normal">
                            {event.displayName}
                          </div>
                          <div className="mt-1 text-center text-xs font-semibold break-words whitespace-normal">
                            {event.reason}
                          </div>
                          <div className="mt-0.5 text-center text-[11px] break-words whitespace-normal text-slate-600">
                            {event.displayTime}
                          </div>
                        </div>
                          );
                        })()
                    ))}
                  </div>
                </>
              ) : null}
            </div>
          </div>
        </div>
        ) : (
        <div>
          <div className="grid grid-cols-7 gap-2 mb-2">
            {DAYS.map((day) => (
              <div key={day} className="text-xs font-black uppercase text-slate-500 px-2 py-1">
                {day}
              </div>
            ))}
          </div>

          <div className="grid grid-cols-7 gap-2 items-stretch">
            {dayCells.map((cell) => (
              <div key={cell.key} className={calendarDayCellClass(cell.date, "desktop")}>
                {cell.date ? (
                  <>
                    <div className="flex items-center justify-between gap-1 mb-2">
                      <span
                        className={[
                          "inline-flex items-center gap-1 text-xs font-black",
                          isTodayDate(cell.date) ? "text-sky-800" : "text-slate-700",
                        ].join(" ")}
                      >
                        {cell.date.getDate()}
                        {isTodayDate(cell.date) ? (
                          <span className="text-[9px] font-black uppercase tracking-wide text-sky-700">Auj.</span>
                        ) : null}
                      </span>
                      {cell.events.length > 0 ? (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            printDaySummary(cell.date!, cell.events, teacherColorIndexMap);
                          }}
                          className="rounded-md p-0.5 text-slate-400 hover:text-slate-700 hover:bg-white/80 transition-colors"
                          aria-label={`Imprimer le résumé du ${cell.date.toLocaleDateString("fr-FR")}`}
                          title="Imprimer le résumé des absences du jour"
                        >
                          <PrinterIcon className="h-3.5 w-3.5" />
                        </button>
                      ) : null}
                    </div>
                    <div className="space-y-1">
                      {cell.events.map((event) => (
                        (() => {
                          const appearance = appearanceForEvent(event, teacherColorIndexMap);
                          return (
                        <div
                          key={event.key}
                          role={event.hasDocument ? "button" : undefined}
                          tabIndex={event.hasDocument ? 0 : -1}
                          onClick={() => (event.hasDocument ? openConvocation(event.id) : undefined)}
                          onKeyDown={(e) => {
                            if (!event.hasDocument) return;
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              openConvocation(event.id);
                            }
                          }}
                          title={
                            event.hasDocument
                              ? event.isOgec
                                ? "Ouvrir le justificatif (PDF)"
                                : "Ouvrir la convocation (PDF)"
                              : event.isOgec
                                ? "Absence sans justificatif"
                                : "Convocation sans document"
                          }
                          style={appearance.cardStyle}
                          className={[
                            "text-left w-full rounded-lg border px-2 py-1 text-[11px] leading-tight",
                            event.hasDocument
                              ? "transition-[filter] cursor-pointer hover:brightness-[0.97]"
                              : "opacity-80 cursor-default",
                            event.hasDocument ? "focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-slate-300" : "",
                          ].join(" ")}
                        >
                          <div className="flex justify-end items-center gap-0.5">
                            {itemsById.get(event.id) && canManageDocsFor(itemsById.get(event.id)!) ? (
                              <button
                                type="button"
                                aria-label="Ajouter un PDF"
                                title="Ajouter un PDF (justificatif)"
                                className="rounded px-0.5 opacity-70 hover:opacity-100"
                                disabled={attachingPdf}
                                onClick={(e) => {
                                  e.preventDefault();
                                  e.stopPropagation();
                                  triggerAttachPdf(event.id);
                                }}
                              >
                                <PaperclipIcon className="h-3 w-3" />
                              </button>
                            ) : null}
                            {event.hasDocument && itemsById.get(event.id) && canManageDocsFor(itemsById.get(event.id)!)
                              ? (event.documentCount > 1
                                  ? Array.from({ length: event.documentCount }, (_, docIdx) => (
                                      <span key={docIdx} className="inline-flex items-center gap-0.5">
                                        <button
                                          type="button"
                                          className="rounded px-0.5 text-[9px] font-bold underline opacity-80 hover:opacity-100"
                                          title={`Ouvrir le PDF ${docIdx + 1}`}
                                          onClick={(e) => {
                                            e.preventDefault();
                                            e.stopPropagation();
                                            openConvocation(event.id, docIdx);
                                          }}
                                        >
                                          {docIdx + 1}
                                        </button>
                                        <button
                                          type="button"
                                          className="rounded px-0.5 text-[9px] font-black text-rose-700 opacity-80 hover:opacity-100"
                                          title={`Supprimer le PDF ${docIdx + 1}`}
                                          onClick={(e) => {
                                            e.preventDefault();
                                            e.stopPropagation();
                                            deleteDocument(event.id, docIdx);
                                          }}
                                        >
                                          ×
                                        </button>
                                      </span>
                                    ))
                                  : (
                                      <button
                                        type="button"
                                        className="rounded px-0.5 text-[9px] font-black text-rose-700 opacity-80 hover:opacity-100"
                                        title="Supprimer le document"
                                        onClick={(e) => {
                                          e.preventDefault();
                                          e.stopPropagation();
                                          deleteDocument(event.id, 0);
                                        }}
                                      >
                                        🗑
                                      </button>
                                    ))
                              : event.documentCount > 1
                                ? Array.from({ length: event.documentCount }, (_, docIdx) => (
                                    <button
                                      key={docIdx}
                                      type="button"
                                      className="rounded px-0.5 text-[9px] font-bold underline opacity-80 hover:opacity-100"
                                      title={`Ouvrir le PDF ${docIdx + 1}`}
                                      onClick={(e) => {
                                        e.preventDefault();
                                        e.stopPropagation();
                                        openConvocation(event.id, docIdx);
                                      }}
                                    >
                                      {docIdx + 1}
                                    </button>
                                  ))
                                : null}
                            {itemsById.get(event.id) && canManageSlot(itemsById.get(event.id)!) ? (
                              <>
                                <button
                                  type="button"
                                  aria-label="Modifier le créneau"
                                  title="Modifier le créneau"
                                  className="rounded px-1 text-[10px] font-black opacity-70 hover:opacity-100"
                                  onClick={(e) => {
                                    e.preventDefault();
                                    e.stopPropagation();
                                    openEdit(event.id);
                                  }}
                                >
                                  ✎
                                </button>
                                <button
                                  type="button"
                                  aria-label="Supprimer le créneau"
                                  title="Supprimer le créneau"
                                  className="rounded px-1 text-[10px] font-black text-rose-700 opacity-70 hover:opacity-100"
                                  onClick={(e) => {
                                    e.preventDefault();
                                    e.stopPropagation();
                                    deleteConvocation(event.id);
                                  }}
                                >
                                  ✕
                                </button>
                              </>
                            ) : null}
                          </div>
                          <div className="font-bold break-words whitespace-normal">{event.displayName}</div>
                          <div className="break-words whitespace-normal">{event.reason}</div>
                          <div className="break-words whitespace-normal">{event.displayTime}</div>
                        </div>
                          );
                        })()
                      ))}
                    </div>
                  </>
                ) : null}
              </div>
            ))}
          </div>
        </div>
        )}
        {loading && <p className="text-sm text-slate-500 mt-3 px-4 sm:px-0">Chargement des absences…</p>}
        {success && (
          <p className="text-sm text-emerald-700 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2 mt-3 mx-4 sm:mx-0">
            {success}
          </p>
        )}
        {error && (
          <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2 mt-3 mx-4 sm:mx-0">
            {error}
          </p>
        )}
      </section>

      {editRecord && editForm ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
          <div className="bg-white rounded-2xl border border-slate-200 shadow-xl w-full max-w-md p-5 space-y-4">
            <h3 className="font-black text-slate-900">Modifier l&apos;absence</h3>
            <p className="text-xs text-slate-500">{editRecord.displayName}</p>
            <label className="block text-sm">
              <span className="font-bold text-slate-700">Nom affiché</span>
              <input
                className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                value={editForm.displayName}
                onChange={(e) => setEditForm({ ...editForm, displayName: e.target.value })}
              />
            </label>
            <label className="block text-sm">
              <span className="font-bold text-slate-700">Motif</span>
              <input
                className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                value={editForm.reason}
                onChange={(e) => setEditForm({ ...editForm, reason: e.target.value })}
              />
            </label>
            <label className="block text-sm">
              <span className="font-bold text-slate-700">Type</span>
              <select
                className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                value={editForm.scope}
                onChange={(e) =>
                  setEditForm({
                    ...editForm,
                    scope: e.target.value as AbsenceScope,
                  })
                }
              >
                <option value="professeur">Professeur</option>
                <option value="ogec">Personnel OGEC</option>
              </select>
            </label>
            {editForm.scope === "professeur" && (
              <label className="block text-sm">
                <span className="font-bold text-slate-700">Établissement</span>
                <select
                  className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                  value={editForm.etablissement}
                  onChange={(e) =>
                    setEditForm({
                      ...editForm,
                      etablissement: e.target.value as Etablissement,
                    })
                  }
                >
                  <option value="École">École</option>
                  <option value="Collège">Collège</option>
                  <option value="Lycée">Lycée</option>
                </select>
              </label>
            )}
            <label className="block text-sm">
              <span className="font-bold text-slate-700">Durée</span>
              <select
                className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                value={editForm.periodType}
                onChange={(e) =>
                  setEditForm({ ...editForm, periodType: e.target.value as AbsencePeriodType })
                }
              >
                <option value="single_day">Une journée (créneau horaire)</option>
                <option value="multi_day">Plusieurs jours</option>
              </select>
            </label>
            {editForm.periodType === "single_day" ? (
              <div className="grid grid-cols-2 gap-2">
                <label className="block text-sm col-span-2">
                  <span className="font-bold text-slate-700">Date</span>
                  <input
                    type="date"
                    className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                    value={editForm.startDate}
                    onChange={(e) => setEditForm({ ...editForm, startDate: e.target.value, endDate: e.target.value })}
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-bold text-slate-700">De</span>
                  <input
                    type="time"
                    className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                    value={editForm.startTime}
                    onChange={(e) => setEditForm({ ...editForm, startTime: e.target.value })}
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-bold text-slate-700">À</span>
                  <input
                    type="time"
                    className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                    value={editForm.endTime}
                    onChange={(e) => setEditForm({ ...editForm, endTime: e.target.value })}
                  />
                </label>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2">
                <label className="block text-sm">
                  <span className="font-bold text-slate-700">Du</span>
                  <input
                    type="date"
                    className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                    value={editForm.startDate}
                    onChange={(e) => setEditForm({ ...editForm, startDate: e.target.value })}
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-bold text-slate-700">Au</span>
                  <input
                    type="date"
                    className="mt-1 w-full border rounded-xl px-3 py-2 text-sm"
                    value={editForm.endDate}
                    onChange={(e) => setEditForm({ ...editForm, endDate: e.target.value })}
                  />
                </label>
              </div>
            )}
            <div className="flex gap-2 justify-end pt-2">
              <button
                type="button"
                className="px-4 py-2 rounded-xl border border-slate-200 text-sm font-bold"
                disabled={editSaving}
                onClick={() => {
                  setEditRecord(null);
                  setEditForm(null);
                }}
              >
                Annuler
              </button>
              <button
                type="button"
                className="px-4 py-2 rounded-xl bg-indigo-600 text-white text-sm font-bold disabled:opacity-50"
                disabled={editSaving}
                onClick={() => void saveEdit()}
              >
                {editSaving ? "Enregistrement…" : "Enregistrer"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
