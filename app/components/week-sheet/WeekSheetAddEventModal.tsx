"use client";

import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { WEEK_DAYS, type WeekDayKey, type WeekSheetData } from "@/app/lib/dashboard-week-sheet-types";

type Props = {
  open: boolean;
  onAdded: (data: WeekSheetData) => void;
  onError: (message: string | null) => void;
  onClose: () => void;
};

const EMPTY = {
  day: "mon" as WeekDayKey,
  startTime: "",
  endTime: "",
  title: "",
  location: "",
};

export default function WeekSheetAddEventModal({ open, onAdded, onError, onClose }: Props) {
  const [mounted, setMounted] = useState(false);
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState(EMPTY);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = "";
      window.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open || !mounted) return null;

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    onError(null);
    try {
      const res = await fetch("/api/dashboard/week-sheet/events", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(form),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Ajout impossible.");
      if (json.data) onAdded(json.data);
      setForm(EMPTY);
      onClose();
    } catch (err) {
      onError(err instanceof Error ? err.message : "Erreur inconnue.");
    } finally {
      setSaving(false);
    }
  };

  return createPortal(
    <div className="fixed inset-0 z-[300] flex items-center justify-center p-4 sm:p-6">
      <button
        type="button"
        className="absolute inset-0 bg-slate-900/25 backdrop-blur-[3px]"
        aria-label="Fermer"
        onClick={onClose}
      />
      <form
        onSubmit={(e) => void submit(e)}
        className="relative z-[1] w-full max-w-lg rounded-2xl border border-[color:var(--dash-border)] bg-white p-5 shadow-2xl shadow-slate-900/15"
        role="dialog"
        aria-modal="true"
        aria-labelledby="week-sheet-add-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-start justify-between gap-3">
          <div>
            <h3 id="week-sheet-add-title" className="text-base font-black text-[#14231A]">
              Ajouter un créneau
            </h3>
            <p className="mt-0.5 text-xs text-stone-500">Compléter la feuille de semaine manuellement.</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg p-1.5 text-stone-400 transition hover:bg-stone-100 hover:text-stone-600"
            aria-label="Fermer"
          >
            ✕
          </button>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <label className="flex flex-col gap-1 text-xs font-bold text-stone-500">
            Jour
            <select
              value={form.day}
              onChange={(e) => setForm((f) => ({ ...f, day: e.target.value as WeekDayKey }))}
              className="rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm font-semibold text-stone-800"
            >
              {WEEK_DAYS.map((d) => (
                <option key={d.key} value={d.key}>
                  {d.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs font-bold text-stone-500">
            Début
            <input
              value={form.startTime}
              onChange={(e) => setForm((f) => ({ ...f, startTime: e.target.value }))}
              placeholder="8h30"
              required
              className="rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm font-semibold text-stone-800"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-bold text-stone-500 sm:col-span-2">
            Fin <span className="font-normal text-stone-400">(optionnel)</span>
            <input
              value={form.endTime}
              onChange={(e) => setForm((f) => ({ ...f, endTime: e.target.value }))}
              placeholder="12h"
              className="rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm font-semibold text-stone-800"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-bold text-stone-500 sm:col-span-2">
            Intitulé
            <input
              value={form.title}
              onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
              placeholder="Conseil de classe, réunion…"
              required
              className="rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm font-semibold text-stone-800"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-bold text-stone-500 sm:col-span-2">
            Lieu <span className="font-normal text-stone-400">(optionnel)</span>
            <input
              value={form.location}
              onChange={(e) => setForm((f) => ({ ...f, location: e.target.value }))}
              placeholder="Salle 12, CDI…"
              className="rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm font-semibold text-stone-800"
            />
          </label>
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="rounded-xl px-4 py-2 text-sm font-bold text-stone-500 hover:bg-stone-50"
          >
            Annuler
          </button>
          <button
            type="submit"
            disabled={saving}
            className="rounded-xl bg-[var(--dash-primary)] px-4 py-2 text-sm font-bold text-white hover:brightness-110 disabled:opacity-60"
          >
            {saving ? "Enregistrement…" : "Ajouter"}
          </button>
        </div>
      </form>
    </div>,
    document.body,
  );
}
