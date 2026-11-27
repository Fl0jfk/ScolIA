"use client";

import { useState } from "react";

export type SantaInfoValues = { eventPlace: string; eventAt: string; budget: string; message: string };

const input = "mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm";
const label = "text-xs font-bold uppercase text-slate-500";

function toLocalInput(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Après le tirage, seules les informations pratiques se modifient. */
export default function SantaInfoForm({
  initial,
  onSubmit,
  onCancel,
}: {
  initial: SantaInfoValues;
  onSubmit: (values: SantaInfoValues) => Promise<void>;
  onCancel: () => void;
}) {
  const [values, setValues] = useState({ ...initial, eventAt: toLocalInput(initial.eventAt) });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const set = (key: keyof SantaInfoValues) => (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
    setValues((v) => ({ ...v, [key]: e.target.value }));

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await onSubmit({ ...values, eventAt: values.eventAt ? new Date(values.eventAt).toISOString() : "" });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Enregistrement impossible.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="grid gap-4 rounded-2xl border border-slate-200 bg-white p-5 sm:grid-cols-2">
      <label className="block">
        <span className={label}>Lieu de la rencontre (facultatif)</span>
        <input className={input} value={values.eventPlace} maxLength={200} onChange={set("eventPlace")} />
      </label>
      <label className="block">
        <span className={label}>Date et heure de la rencontre (facultatif)</span>
        <input type="datetime-local" className={input} value={values.eventAt} onChange={set("eventAt")} />
      </label>
      <label className="block sm:col-span-2">
        <span className={label}>Budget (facultatif)</span>
        <input className={input} value={values.budget} maxLength={80} onChange={set("budget")} />
      </label>
      <label className="block sm:col-span-2">
        <span className={label}>Message aux participants (facultatif)</span>
        <textarea className={`${input} min-h-[70px]`} value={values.message} maxLength={1000} onChange={set("message")} />
      </label>
      {error && (
        <p role="alert" className="text-sm text-rose-700 sm:col-span-2">
          {error}
        </p>
      )}
      <div className="flex items-center gap-3 sm:col-span-2">
        <button type="submit" disabled={busy} className="rounded-xl bg-red-600 px-5 py-2 text-sm font-bold text-white disabled:opacity-50">
          {busy ? "Enregistrement…" : "Enregistrer"}
        </button>
        <button type="button" onClick={onCancel} className="text-sm font-bold text-slate-500 underline">
          Annuler
        </button>
      </div>
    </form>
  );
}
