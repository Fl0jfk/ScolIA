"use client";

import { useMemo, useState } from "react";
import {
  estimateAnnualBalance,
  type AnnualBalanceEstimate,
  type PlanningDayException,
  type PlanningExceptionKind,
  type StaffPlanningDoc,
  type TeacherPlanningDoc,
  type TeacherReplacementSlot,
} from "@/app/lib/rh/planning-types";

function newId(prefix: string) {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
}

export function StaffQuotaBanner({
  staff,
  canEdit,
  onChangeTarget,
}: {
  staff: StaffPlanningDoc;
  canEdit: boolean;
  onChangeTarget: (hours: number | undefined) => void;
}) {
  if (staff.mode !== "fixed") return null;
  const balance = estimateAnnualBalance(staff);
  return (
    <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 space-y-2">
      <p className="text-xs font-black uppercase tracking-wide text-slate-500">
        Semaine type annuelle · quota
      </p>
      <div className="flex flex-wrap items-end gap-3">
        <label className="text-xs font-bold text-slate-600">
          Quota annuel (h)
          <input
            type="number"
            min={0}
            max={3000}
            step={0.5}
            disabled={!canEdit}
            className="mt-0.5 block w-28 border rounded-lg px-2 py-1.5 text-sm disabled:bg-slate-100"
            value={staff.annualHoursTarget ?? ""}
            placeholder="ex. 1607"
            onChange={(e) => {
              const v = e.target.value.trim();
              if (!v) onChangeTarget(undefined);
              else onChangeTarget(Number(v));
            }}
          />
        </label>
        <BalanceText balance={balance} />
      </div>
    </div>
  );
}

function BalanceText({ balance }: { balance: AnnualBalanceEstimate }) {
  return (
    <div className="text-xs text-slate-600 space-y-0.5">
      <p>
        Semaine type : <span className="font-bold">{balance.weeklyHours} h</span>
        {" · "}
        Projeté ({balance.weeksFactor} sem.) :{" "}
        <span className="font-bold">{balance.projectedAnnualHours} h</span>
      </p>
      {balance.exceptionDeltaHours !== 0 ? (
        <p>
          Delta exceptions :{" "}
          <span className="font-bold">
            {balance.exceptionDeltaHours > 0 ? "+" : ""}
            {balance.exceptionDeltaHours} h
          </span>
        </p>
      ) : null}
      {balance.balanceHours != null ? (
        <p className={balance.balanceHours > 0 ? "text-amber-800 font-bold" : "text-emerald-800 font-bold"}>
          Solde estimé vs quota : {balance.balanceHours > 0 ? "+" : ""}
          {balance.balanceHours} h
          {balance.balanceHours > 0 ? " (avance / dépassement)" : " (reste à faire)"}
        </p>
      ) : (
        <p className="text-slate-400 italic">Renseignez un quota pour voir le solde.</p>
      )}
    </div>
  );
}

export function StaffExceptionsPanel({
  staff,
  canEdit,
  onChange,
}: {
  staff: StaffPlanningDoc;
  canEdit: boolean;
  onChange: (exceptions: PlanningDayException[]) => void;
}) {
  if (staff.mode !== "fixed") return null;

  const [date, setDate] = useState("");
  const [start, setStart] = useState("08:00");
  const [end, setEnd] = useState("17:00");
  const [kind, setKind] = useState<PlanningExceptionKind>("avance");
  const [note, setNote] = useState("");

  const sorted = useMemo(
    () => [...(staff.exceptions || [])].sort((a, b) => b.date.localeCompare(a.date)),
    [staff.exceptions],
  );

  const add = () => {
    if (!canEdit || !date || start >= end) return;
    const row: PlanningDayException = {
      id: newId("exc"),
      date,
      start,
      end,
      kind,
      note: note.trim() || undefined,
      createdBy: "self",
      createdAt: new Date().toISOString(),
    };
    onChange([row, ...sorted]);
    setNote("");
  };

  return (
    <div className="rounded-xl border border-amber-100 bg-amber-50/50 p-3 space-y-3">
      <div>
        <p className="text-xs font-black uppercase tracking-wide text-amber-800">Ajuster un jour</p>
        <p className="text-[11px] text-amber-900/80 mt-0.5">
          Ex. arriver / partir plus tôt (avance sur le quota annuel) — pas forcément des heures payées.
        </p>
      </div>
      {canEdit ? (
        <div className="grid grid-cols-2 sm:grid-cols-5 gap-2 items-end">
          <label className="text-[11px] font-bold text-slate-600">
            Date
            <input
              type="date"
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Début
            <input
              type="time"
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={start}
              onChange={(e) => setStart(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Fin
            <input
              type="time"
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={end}
              onChange={(e) => setEnd(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Motif
            <select
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={kind}
              onChange={(e) => setKind(e.target.value as PlanningExceptionKind)}
            >
              <option value="avance">Avance</option>
              <option value="ajustement">Ajustement</option>
              <option value="depassement">Dépassement</option>
            </select>
          </label>
          <button
            type="button"
            onClick={add}
            className="px-3 py-1.5 rounded-xl text-xs font-bold bg-amber-700 text-white hover:bg-amber-800"
          >
            Ajouter
          </button>
          <label className="col-span-2 sm:col-span-5 text-[11px] font-bold text-slate-600">
            Note
            <input
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={note}
              placeholder="Ex. canicule — décalage −30 min"
              onChange={(e) => setNote(e.target.value)}
            />
          </label>
        </div>
      ) : null}
      {sorted.length === 0 ? (
        <p className="text-xs text-slate-400 italic">Aucune exception déclarée.</p>
      ) : (
        <ul className="space-y-1.5">
          {sorted.slice(0, 12).map((e) => (
            <li
              key={e.id}
              className="flex flex-wrap items-center gap-2 text-xs bg-white border border-amber-100 rounded-lg px-2 py-1.5"
            >
              <span className="font-bold text-slate-800">{e.date}</span>
              <span>
                {e.start}–{e.end}
              </span>
              <span className="uppercase tracking-wide text-[10px] font-black text-amber-800">
                {e.kind}
              </span>
              {e.note ? <span className="text-slate-500">{e.note}</span> : null}
              {canEdit ? (
                <button
                  type="button"
                  className="ml-auto text-rose-500 font-bold"
                  onClick={() => onChange(sorted.filter((x) => x.id !== e.id))}
                >
                  ✕
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function TeacherReplacementsPanel({
  teacher,
  canManage,
  onChange,
}: {
  teacher: TeacherPlanningDoc;
  canManage: boolean;
  onChange: (replacements: TeacherReplacementSlot[]) => void;
}) {
  const [date, setDate] = useState("");
  const [start, setStart] = useState("08:00");
  const [end, setEnd] = useState("09:00");
  const [subject, setSubject] = useState("");
  const [classes, setClasses] = useState("");
  const [room, setRoom] = useState("");
  const [note, setNote] = useState("");

  const sorted = useMemo(
    () => [...(teacher.replacements || [])].sort((a, b) => a.date.localeCompare(b.date)),
    [teacher.replacements],
  );

  const add = () => {
    if (!canManage || !date || !subject.trim() || start >= end) return;
    const row: TeacherReplacementSlot = {
      id: newId("repl"),
      date,
      start,
      end,
      subject: subject.trim(),
      classes: classes
        .split(",")
        .map((c) => c.trim())
        .filter(Boolean),
      room: room.trim() || undefined,
      note: note.trim() || undefined,
      createdBy: "rh",
      createdAt: new Date().toISOString(),
    };
    onChange([...sorted, row]);
    setSubject("");
    setClasses("");
    setRoom("");
    setNote("");
  };

  return (
    <div className="rounded-xl border border-violet-100 bg-violet-50/50 p-3 space-y-3">
      <div>
        <p className="text-xs font-black uppercase tracking-wide text-violet-800">Remplacements</p>
        <p className="text-[11px] text-violet-900/80 mt-0.5">
          Créneaux datés en plus des semaines types A/B — visibles sur le planning du professeur.
        </p>
      </div>
      {canManage ? (
        <div className="grid grid-cols-2 sm:grid-cols-6 gap-2 items-end">
          <label className="text-[11px] font-bold text-slate-600">
            Date
            <input
              type="date"
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Début
            <input
              type="time"
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={start}
              onChange={(e) => setStart(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Fin
            <input
              type="time"
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={end}
              onChange={(e) => setEnd(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Matière
            <input
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
            />
          </label>
          <label className="text-[11px] font-bold text-slate-600">
            Classe(s)
            <input
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              placeholder="6A"
              value={classes}
              onChange={(e) => setClasses(e.target.value)}
            />
          </label>
          <button
            type="button"
            onClick={add}
            className="px-3 py-1.5 rounded-xl text-xs font-bold bg-violet-700 text-white hover:bg-violet-800"
          >
            Ajouter
          </button>
          <label className="col-span-2 sm:col-span-3 text-[11px] font-bold text-slate-600">
            Salle
            <input
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={room}
              onChange={(e) => setRoom(e.target.value)}
            />
          </label>
          <label className="col-span-2 sm:col-span-3 text-[11px] font-bold text-slate-600">
            Note
            <input
              className="mt-0.5 w-full border rounded-lg px-2 py-1.5 text-sm"
              value={note}
              placeholder="Remplacement M. Dupont"
              onChange={(e) => setNote(e.target.value)}
            />
          </label>
        </div>
      ) : (
        <p className="text-[11px] text-slate-500">Seule la RH peut ajouter des remplacements.</p>
      )}
      {sorted.length === 0 ? (
        <p className="text-xs text-slate-400 italic">Aucun remplacement planifié.</p>
      ) : (
        <ul className="space-y-1.5">
          {sorted.map((r) => (
            <li
              key={r.id}
              className="flex flex-wrap items-center gap-2 text-xs bg-white border border-violet-100 rounded-lg px-2 py-1.5"
            >
              <span className="font-bold text-slate-800">{r.date}</span>
              <span>
                {r.start}–{r.end}
              </span>
              <span className="font-semibold">{r.subject}</span>
              <span className="text-slate-500">{r.classes.join(", ")}</span>
              {r.room ? <span className="text-slate-400">{r.room}</span> : null}
              {r.note ? <span className="text-slate-500 italic">{r.note}</span> : null}
              {canManage ? (
                <button
                  type="button"
                  className="ml-auto text-rose-500 font-bold"
                  onClick={() => onChange(sorted.filter((x) => x.id !== r.id))}
                >
                  ✕
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function DayFocusBanner({
  date,
  onDateChange,
  exception,
  replacements,
}: {
  date: string;
  onDateChange: (d: string) => void;
  exception?: PlanningDayException | null;
  replacements: TeacherReplacementSlot[];
}) {
  return (
    <div className="rounded-xl border border-slate-200 bg-white px-3 py-2 flex flex-wrap items-center gap-3">
      <label className="text-[11px] font-bold text-slate-600">
        Focus jour
        <input
          type="date"
          className="mt-0.5 block border rounded-lg px-2 py-1.5 text-sm"
          value={date}
          onChange={(e) => onDateChange(e.target.value)}
        />
      </label>
      {exception ? (
        <p className="text-xs text-amber-800 font-semibold">
          Exception ce jour : {exception.start}–{exception.end} ({exception.kind})
        </p>
      ) : null}
      {replacements.length > 0 ? (
        <p className="text-xs text-violet-800 font-semibold">
          {replacements.length} remplacement{replacements.length > 1 ? "s" : ""} ce jour
        </p>
      ) : null}
    </div>
  );
}
