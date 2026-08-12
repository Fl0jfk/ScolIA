"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  InternatInstallationBooking,
  InternatInstallationConfig,
  InternatInstallationDay,
} from "@/app/lib/internat-types";
import { formatInstallationSlotFr } from "@/app/lib/internat-installation-slots";

const emptyDay = (): InternatInstallationDay => ({
  date: "",
  openTime: "09:00",
  closeTime: "17:00",
});

export default function InternatInstallationPanel({ canManage }: { canManage: boolean }) {
  const [config, setConfig] = useState<InternatInstallationConfig | null>(null);
  const [bookings, setBookings] = useState<InternatInstallationBooking[]>([]);
  const [generatedSlotCount, setGeneratedSlotCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const publicUrl =
    typeof window !== "undefined"
      ? `${window.location.origin}/internat/installation`
      : "/internat/installation";

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/internat/installation", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setConfig(data.config);
      setBookings(data.bookings || []);
      setGeneratedSlotCount(data.generatedSlotCount || 0);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const bookingsByDay = useMemo(() => {
    const map = new Map<string, InternatInstallationBooking[]>();
    for (const b of bookings) {
      const date = b.slotStart.slice(0, 10);
      const list = map.get(date) ?? [];
      list.push(b);
      map.set(date, list);
    }
    return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [bookings]);

  const save = async () => {
    if (!config || !canManage) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/internat/installation", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      setConfig(data.config);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const cancelBooking = async (id: string) => {
    if (!canManage || !confirm("Annuler cette inscription ?")) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/internat/installation?id=${encodeURIComponent(id)}`, {
        method: "DELETE",
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Suppression impossible");
      await load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const copyLink = async () => {
    try {
      await navigator.clipboard.writeText(publicUrl);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      alert(publicUrl);
    }
  };

  if (loading || !config) {
    return <p className="text-sm text-slate-500">Chargement…</p>;
  }

  return (
    <div className="space-y-8">
      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/60 p-5 text-sm text-indigo-950">
        <p className="font-bold mb-1">Installation / prise de rendez-vous</p>
        <p>
          Lien public à envoyer aux parents (sans token). Configurez les jours et horaires, puis
          activez la page.
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <code className="rounded-lg bg-white/80 px-2 py-1 text-xs break-all">{publicUrl}</code>
          <button
            type="button"
            onClick={() => void copyLink()}
            className="rounded-full border border-indigo-200 bg-white px-3 py-1 text-xs font-semibold text-indigo-800 hover:bg-indigo-50"
          >
            {copied ? "Copié" : "Copier le lien"}
          </button>
        </div>
      </div>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700">
          {error}
        </p>
      ) : null}

      <section className="space-y-4 rounded-2xl border border-slate-200 bg-white p-5">
        <h3 className="font-black text-slate-900">Réglages</h3>
        <label className="flex items-center gap-2 text-sm text-slate-700">
          <input
            type="checkbox"
            checked={config.enabled}
            disabled={!canManage}
            onChange={(e) => setConfig({ ...config, enabled: e.target.checked })}
          />
          Page publique active
        </label>
        <label className="block space-y-1 text-sm">
          <span className="text-slate-600">Titre</span>
          <input
            value={config.title}
            disabled={!canManage}
            onChange={(e) => setConfig({ ...config, title: e.target.value })}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 disabled:bg-slate-50"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="text-slate-600">Message d’accueil</span>
          <textarea
            value={config.intro || ""}
            disabled={!canManage}
            rows={3}
            onChange={(e) => setConfig({ ...config, intro: e.target.value })}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 disabled:bg-slate-50"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="text-slate-600">Lieu (mail / agenda)</span>
          <input
            value={config.location || ""}
            disabled={!canManage}
            onChange={(e) => setConfig({ ...config, location: e.target.value })}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 disabled:bg-slate-50"
          />
        </label>
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="block space-y-1 text-sm">
            <span className="text-slate-600">Durée d’un créneau (min)</span>
            <input
              type="number"
              min={5}
              max={180}
              value={config.slotDurationMinutes}
              disabled={!canManage}
              onChange={(e) =>
                setConfig({ ...config, slotDurationMinutes: Number(e.target.value) || 30 })
              }
              className="w-full rounded-xl border border-slate-200 px-3 py-2 disabled:bg-slate-50"
            />
          </label>
          <label className="block space-y-1 text-sm">
            <span className="text-slate-600">Familles max / créneau</span>
            <input
              type="number"
              min={1}
              max={50}
              value={config.maxFamiliesPerSlot}
              disabled={!canManage}
              onChange={(e) =>
                setConfig({ ...config, maxFamiliesPerSlot: Number(e.target.value) || 1 })
              }
              className="w-full rounded-xl border border-slate-200 px-3 py-2 disabled:bg-slate-50"
            />
          </label>
        </div>
        <p className="text-xs text-slate-500">
          {generatedSlotCount} créneau{generatedSlotCount !== 1 ? "x" : ""} généré
          {generatedSlotCount !== 1 ? "s" : ""} avec la config actuelle.
        </p>

        <div className="space-y-3 border-t border-slate-100 pt-4">
          <div className="flex items-center justify-between gap-2">
            <h4 className="font-bold text-slate-900 text-sm">Jours d’ouverture</h4>
            {canManage ? (
              <button
                type="button"
                onClick={() => setConfig({ ...config, days: [...config.days, emptyDay()] })}
                className="text-xs font-semibold text-indigo-700 hover:underline"
              >
                + Ajouter un jour
              </button>
            ) : null}
          </div>
          {!config.days.length ? (
            <p className="text-xs text-slate-500">Aucun jour configuré.</p>
          ) : (
            config.days.map((day, idx) => (
              <div
                key={`${day.date}-${idx}`}
                className="grid gap-2 rounded-xl border border-slate-100 bg-slate-50/80 p-3 sm:grid-cols-4"
              >
                <label className="block space-y-1 text-xs sm:col-span-2">
                  <span className="text-slate-500">Date</span>
                  <input
                    type="date"
                    value={day.date}
                    disabled={!canManage}
                    onChange={(e) => {
                      const days = [...config.days];
                      days[idx] = { ...day, date: e.target.value };
                      setConfig({ ...config, days });
                    }}
                    className="w-full rounded-lg border border-slate-200 px-2 py-1.5 disabled:bg-white"
                  />
                </label>
                <label className="block space-y-1 text-xs">
                  <span className="text-slate-500">Début</span>
                  <input
                    type="time"
                    value={day.openTime}
                    disabled={!canManage}
                    onChange={(e) => {
                      const days = [...config.days];
                      days[idx] = { ...day, openTime: e.target.value };
                      setConfig({ ...config, days });
                    }}
                    className="w-full rounded-lg border border-slate-200 px-2 py-1.5 disabled:bg-white"
                  />
                </label>
                <label className="block space-y-1 text-xs">
                  <span className="text-slate-500">Fin</span>
                  <div className="flex gap-1">
                    <input
                      type="time"
                      value={day.closeTime}
                      disabled={!canManage}
                      onChange={(e) => {
                        const days = [...config.days];
                        days[idx] = { ...day, closeTime: e.target.value };
                        setConfig({ ...config, days });
                      }}
                      className="w-full rounded-lg border border-slate-200 px-2 py-1.5 disabled:bg-white"
                    />
                    {canManage ? (
                      <button
                        type="button"
                        aria-label="Supprimer le jour"
                        onClick={() =>
                          setConfig({
                            ...config,
                            days: config.days.filter((_, i) => i !== idx),
                          })
                        }
                        className="rounded-lg px-2 text-slate-400 hover:bg-white hover:text-rose-600"
                      >
                        ×
                      </button>
                    ) : null}
                  </div>
                </label>
              </div>
            ))
          )}
        </div>

        {canManage ? (
          <button
            type="button"
            disabled={busy}
            onClick={() => void save()}
            className="rounded-full bg-indigo-700 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-800 disabled:opacity-50"
          >
            {busy ? "Enregistrement…" : "Enregistrer les réglages"}
          </button>
        ) : null}
      </section>

      <section className="space-y-4">
        <h3 className="font-black text-slate-900">Inscriptions</h3>
        {!bookings.length ? (
          <p className="text-sm text-slate-500">Aucune inscription pour le moment.</p>
        ) : (
          bookingsByDay.map(([date, rows]) => (
            <div key={date} className="rounded-2xl border border-slate-200 bg-white overflow-hidden">
              <div className="border-b border-slate-100 bg-slate-50 px-4 py-2 text-sm font-bold text-slate-800">
                {new Date(`${date}T12:00:00`).toLocaleDateString("fr-FR", {
                  weekday: "long",
                  day: "numeric",
                  month: "long",
                  year: "numeric",
                })}
                <span className="ml-2 font-normal text-slate-500">
                  ({rows.length} RDV)
                </span>
              </div>
              <ul className="divide-y divide-slate-100">
                {rows.map((b) => (
                  <li
                    key={b.id}
                    className="flex flex-wrap items-start justify-between gap-2 px-4 py-3 text-sm"
                  >
                    <div>
                      <p className="font-semibold text-slate-900">
                        {b.slotStart.slice(11)} — {b.studentFirstName} {b.studentLastName}
                      </p>
                      <p className="text-xs text-slate-500">
                        {b.parentPhone} · {b.parentEmail}
                      </p>
                      <p className="text-[11px] text-slate-400 mt-0.5">
                        {formatInstallationSlotFr(b.slotStart)}
                      </p>
                    </div>
                    {canManage ? (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void cancelBooking(b.id)}
                        className="text-xs font-semibold text-rose-600 hover:underline disabled:opacity-50"
                      >
                        Annuler
                      </button>
                    ) : null}
                  </li>
                ))}
              </ul>
            </div>
          ))
        )}
      </section>
    </div>
  );
}
