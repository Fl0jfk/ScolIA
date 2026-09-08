"use client";

import { useMemo, useState } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import {
  PORTES_OUVERTES_CYCLE_LABELS,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot, PortesOuvertesToolConfig } from "@/app/lib/toolbox-types";

type PublicSlot = PortesOuvertesSlot & {
  registeredCount: number;
  remaining: number | null;
};

type PublicConfig = Omit<PortesOuvertesToolConfig, "slots"> & {
  slots: PublicSlot[];
};

function slotsForCycle(slots: PublicSlot[], cycle: PortesOuvertesCycle): PublicSlot[] {
  return slots.filter((s) => !s.cycle || s.cycle === cycle);
}

function firstOpenSlotId(slots: PublicSlot[], cycle: PortesOuvertesCycle): string {
  const list = slotsForCycle(slots, cycle);
  const open = list.find((s) => s.remaining === null || s.remaining > 0);
  return open?.id ?? list[0]?.id ?? "";
}

function formatSlotLabel(s: PublicSlot): string {
  const when = new Date(s.startAt).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "medium",
    timeStyle: "short",
  });
  if (s.remaining === 0) return `${s.label} — ${when} — complet`;
  if (s.remaining === null) return `${s.label} — ${when}`;
  return `${s.label} — ${when} (${s.remaining} place${s.remaining > 1 ? "s" : ""} restante${s.remaining > 1 ? "s" : ""})`;
}

export default function PortesOuvertesClient({
  po,
  cycles,
  cycleLabels,
}: {
  po: PublicConfig;
  cycles: PortesOuvertesCycle[];
  cycleLabels: Partial<Record<PortesOuvertesCycle, string>>;
}) {
  const availableCycles = cycles.length > 0 ? cycles : (["college"] as PortesOuvertesCycle[]);
  const initialCycle = availableCycles[0];
  const [form, setForm] = useState({
    slotId: firstOpenSlotId(po.slots, initialCycle),
    firstName: "",
    lastName: "",
    email: "",
    phone: "",
    cycle: initialCycle,
    childFirstName: "",
    childLastName: "",
    classeSouhaitee: "",
  });
  const [honeypot, setHoneypot] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const cycleSlots = useMemo(
    () => slotsForCycle(po.slots, form.cycle),
    [form.cycle, po.slots],
  );
  const selectedSlot = cycleSlots.find((s) => s.id === form.slotId);
  const selectedFull = selectedSlot?.remaining === 0;
  const allSlotsFull =
    cycleSlots.length > 0 && cycleSlots.every((s) => s.remaining === 0);

  function setCycle(cycle: PortesOuvertesCycle) {
    setForm({
      ...form,
      cycle,
      slotId: firstOpenSlotId(po.slots, cycle),
    });
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (selectedFull || allSlotsFull) {
      setError("Ce créneau est complet. Veuillez en choisir un autre.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/portes-ouvertes/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          slotId: form.slotId,
          firstName: form.firstName,
          lastName: form.lastName,
          email: form.email,
          phone: form.phone,
          cycle: form.cycle,
          childFirstName: form.childFirstName,
          childLastName: form.childLastName,
          classeSouhaitee: form.classeSouhaitee,
          website: honeypot,
        }),
      });
      const j = (await res.json()) as { error?: string };
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

          {error && (
            <p className="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
              {error}
            </p>
          )}

          {po.slots.length === 0 ? (
            <p className="mt-8 text-sm text-slate-500">
              Les créneaux d&apos;inscription seront bientôt publiés.
            </p>
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

              <fieldset className="space-y-4 rounded-2xl border border-violet-100 bg-violet-50/40 p-4">
                <legend className="px-1 text-xs font-bold uppercase tracking-wide text-violet-700">
                  Établissement et créneau
                </legend>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Établissement</span>
                  <select
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm font-semibold"
                    value={form.cycle}
                    onChange={(e) => setCycle(e.target.value as PortesOuvertesCycle)}
                    required
                  >
                    {availableCycles.map((c) => (
                      <option key={c} value={c}>
                        {cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                      </option>
                    ))}
                  </select>
                </label>
                {cycleSlots.length === 0 ? (
                  <p className="text-sm text-amber-800">
                    Aucun créneau publié pour{" "}
                    {cycleLabels[form.cycle] || PORTES_OUVERTES_CYCLE_LABELS[form.cycle]}.
                  </p>
                ) : allSlotsFull ? (
                  <p className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                    Tous les créneaux sont complets pour cet établissement.
                  </p>
                ) : (
                  <label className="block">
                    <span className="text-xs font-bold uppercase text-slate-500">Créneau</span>
                    <select
                      className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm font-semibold"
                      value={form.slotId}
                      onChange={(e) => setForm({ ...form, slotId: e.target.value })}
                      required
                    >
                      {cycleSlots.map((s) => (
                        <option key={s.id} value={s.id} disabled={s.remaining === 0}>
                          {formatSlotLabel(s)}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </fieldset>

              {!allSlotsFull && cycleSlots.length > 0 ? (
                <>
                  <fieldset className="space-y-4 rounded-2xl border border-slate-100 bg-slate-50/70 p-4">
                    <legend className="px-1 text-xs font-bold uppercase tracking-wide text-slate-500">
                      Vos coordonnées
                    </legend>
                    <div className="grid gap-4 sm:grid-cols-2">
                      <label className="block">
                        <span className="text-xs font-bold uppercase text-slate-500">Prénom</span>
                        <input
                          required
                          className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                          value={form.firstName}
                          onChange={(e) => setForm({ ...form, firstName: e.target.value })}
                          autoComplete="given-name"
                        />
                      </label>
                      <label className="block">
                        <span className="text-xs font-bold uppercase text-slate-500">Nom</span>
                        <input
                          required
                          className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                          value={form.lastName}
                          onChange={(e) => setForm({ ...form, lastName: e.target.value })}
                          autoComplete="family-name"
                        />
                      </label>
                    </div>
                    <label className="block">
                      <span className="text-xs font-bold uppercase text-slate-500">E-mail</span>
                      <input
                        required
                        type="email"
                        className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                        value={form.email}
                        onChange={(e) => setForm({ ...form, email: e.target.value })}
                        autoComplete="email"
                      />
                    </label>
                    <label className="block">
                      <span className="text-xs font-bold uppercase text-slate-500">
                        Téléphone (optionnel)
                      </span>
                      <input
                        className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                        value={form.phone}
                        onChange={(e) => setForm({ ...form, phone: e.target.value })}
                        autoComplete="tel"
                      />
                    </label>
                  </fieldset>

                  <fieldset className="space-y-4 rounded-2xl border border-violet-100 bg-violet-50/40 p-4">
                    <legend className="px-1 text-xs font-bold uppercase tracking-wide text-violet-700">
                      Enfant concerné
                    </legend>
                    <div className="grid gap-4 sm:grid-cols-2">
                      <label className="block">
                        <span className="text-xs font-bold uppercase text-slate-500">
                          Prénom de l’enfant
                        </span>
                        <input
                          required
                          className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                          value={form.childFirstName}
                          onChange={(e) => setForm({ ...form, childFirstName: e.target.value })}
                        />
                      </label>
                      <label className="block">
                        <span className="text-xs font-bold uppercase text-slate-500">
                          Nom de l’enfant
                        </span>
                        <input
                          required
                          className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                          value={form.childLastName}
                          onChange={(e) => setForm({ ...form, childLastName: e.target.value })}
                        />
                      </label>
                    </div>
                    <label className="block">
                      <span className="text-xs font-bold uppercase text-slate-500">
                        Classe / niveau souhaité
                      </span>
                      <input
                        required
                        className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                        value={form.classeSouhaitee}
                        onChange={(e) => setForm({ ...form, classeSouhaitee: e.target.value })}
                        placeholder="Ex. cinquième, quatrième, 6e…"
                      />
                    </label>
                  </fieldset>

                  <button
                    type="submit"
                    disabled={busy || selectedFull}
                    className="w-full rounded-xl bg-violet-600 py-3 text-sm font-bold text-white hover:bg-violet-700 disabled:opacity-50"
                  >
                    {busy ? "Inscription…" : "Confirmer mon inscription"}
                  </button>
                </>
              ) : null}
            </form>
          )}
        </div>
      </main>
    </>
  );
}
