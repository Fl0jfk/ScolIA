"use client";

import { useState } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import type { PortesOuvertesToolConfig } from "@/app/lib/toolbox-types";

export default function PortesOuvertesClient({ po }: { po: PortesOuvertesToolConfig }) {
  const [form, setForm] = useState({
    slotId: po.slots[0]?.id ?? "",
    firstName: "",
    lastName: "",
    email: "",
    phone: "",
    childrenInfo: "",
    consent: false,
  });
  const [honeypot, setHoneypot] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/portes-ouvertes/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...form, website: honeypot }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setDone(true);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (done) {
    return (
      <>
        <RentreePublicHeader />
        <main className="mx-auto max-w-lg px-4 py-16">
          <div className="rounded-3xl border border-emerald-200 bg-emerald-50 p-8 text-center">
            <h1 className="text-2xl font-black text-emerald-900">Inscription confirmée</h1>
            <p className="mt-4 text-sm text-emerald-800">
              Un e-mail de confirmation vous a été envoyé avec un fichier calendrier (.ics) à ajouter à votre agenda.
            </p>
          </div>
        </main>
      </>
    );
  }

  return (
    <>
      <RentreePublicHeader />
      <main className="mx-auto max-w-xl px-4 py-8">
        <div className="rounded-3xl border border-violet-200 bg-white p-6 md:p-8 shadow-xl">
          <h1 className="text-3xl font-black text-violet-900">{po.title}</h1>
          <p className="mt-3 text-sm text-slate-600">{po.intro}</p>
          {po.address && (
            <p className="mt-4 text-sm font-semibold text-slate-800">
              📍 {po.address}
              {po.mapsUrl && (
                <>
                  {" "}
                  —{" "}
                  <a href={po.mapsUrl} target="_blank" rel="noopener noreferrer" className="text-violet-700 underline">
                    Itinéraire
                  </a>
                </>
              )}
            </p>
          )}

          {error && <p className="mt-4 rounded-xl bg-rose-50 border border-rose-200 px-4 py-3 text-sm text-rose-800">{error}</p>}

          {po.slots.length === 0 ? (
            <p className="mt-8 text-sm text-slate-500">Les créneaux d&apos;inscription seront bientôt publiés.</p>
          ) : (
            <form onSubmit={(e) => void submit(e)} className="relative mt-8 space-y-4">
              <div
                className="pointer-events-none absolute left-0 top-0 -z-10 h-0 w-0 overflow-hidden opacity-0"
                aria-hidden
              >
                <label>
                  Site web
                  <input
                    tabIndex={-1}
                    autoComplete="off"
                    value={honeypot}
                    onChange={(e) => setHoneypot(e.target.value)}
                  />
                </label>
              </div>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Créneau</span>
                <select
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                  value={form.slotId}
                  onChange={(e) => setForm({ ...form, slotId: e.target.value })}
                  required
                >
                  {po.slots.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.label} — {new Date(s.startAt).toLocaleString("fr-FR", { dateStyle: "medium", timeStyle: "short" })}
                    </option>
                  ))}
                </select>
              </label>
              <div className="grid gap-4 sm:grid-cols-2">
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Prénom</span>
                  <input required className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={form.firstName} onChange={(e) => setForm({ ...form, firstName: e.target.value })} />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Nom</span>
                  <input required className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={form.lastName} onChange={(e) => setForm({ ...form, lastName: e.target.value })} />
                </label>
              </div>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">E-mail</span>
                <input required type="email" className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Téléphone (optionnel)</span>
                <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Enfant(s) / classe(s) concerné(s)</span>
                <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={form.childrenInfo} onChange={(e) => setForm({ ...form, childrenInfo: e.target.value })} placeholder="Ex. Emma — 6e A" />
              </label>
              <label className="flex items-start gap-3 cursor-pointer">
                <input type="checkbox" required checked={form.consent} onChange={(e) => setForm({ ...form, consent: e.target.checked })} className="mt-1" />
                <span className="text-xs text-slate-600">{po.consentLabel}</span>
              </label>
              <button
                type="submit"
                disabled={busy}
                className="w-full rounded-xl bg-violet-600 py-3 text-sm font-bold text-white hover:bg-violet-700 disabled:opacity-50"
              >
                {busy ? "Inscription…" : "Confirmer mon inscription"}
              </button>
            </form>
          )}
        </div>
      </main>
    </>
  );
}
