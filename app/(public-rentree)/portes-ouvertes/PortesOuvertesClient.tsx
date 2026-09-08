"use client";

import { useMemo, useState } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import { parisDateKey, parseParisDateTime } from "@/app/lib/paris-time";
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

function formatHm(iso: string): string {
  return new Date(iso).toLocaleTimeString("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDayLong(dayKey: string): string {
  const d = parseParisDateTime(dayKey, "12:00");
  if (!d) return dayKey;
  return d.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

/** Dates des créneaux, pour le titre « Portes ouvertes du … ». */
function daysLabelFromSlots(slots: PublicSlot[]): string | null {
  if (!slots.length) return null;
  const days = [...new Set(slots.map((s) => parisDateKey(s.startAt)))].sort();
  if (days.length === 1) return formatDayLong(days[0]!);
  if (days.length === 2) return `${formatDayLong(days[0]!)} et ${formatDayLong(days[1]!)}`;
  return `${formatDayLong(days[0]!)} → ${formatDayLong(days[days.length - 1]!)}`;
}

function pageHeading(baseTitle: string, dayLabel: string | null): string {
  const title = (baseTitle || "Portes ouvertes").trim();
  if (!dayLabel) return title;
  // Évite « Portes ouvertes du … du … » si le titre admin contient déjà une date.
  if (/\bdu\s+\d/i.test(title) || /\d{1,2}\s+\p{L}+/u.test(title)) return title;
  return `${title} du ${dayLabel}`;
}

function placesLabel(remaining: number | null): string | null {
  if (remaining === null) return null;
  if (remaining === 0) return "Complet";
  return `${remaining} place${remaining > 1 ? "s" : ""}`;
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
  const dayLabel = useMemo(() => daysLabelFromSlots(cycleSlots), [cycleSlots]);
  const heading = pageHeading(po.title, dayLabel);
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
    if (!form.slotId) {
      setError("Veuillez choisir un créneau.");
      return;
    }
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

  const fieldClass =
    "mt-1.5 w-full rounded-2xl border border-slate-200/90 bg-white px-3.5 py-2.5 text-sm text-slate-900 shadow-sm outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-200";

  if (done) {
    return (
      <div className="min-h-screen bg-[linear-gradient(180deg,#f5f3ff_0%,#f8fafc_42%,#ffffff_100%)]">
        <RentreePublicHeader />
        <main className="mx-auto max-w-2xl px-4 py-16 sm:px-6">
          <div className="rounded-[1.75rem] border border-emerald-200/80 bg-white p-8 text-center shadow-[0_24px_60px_-36px_rgba(16,185,129,0.55)] sm:p-10">
            <p className="text-xs font-bold uppercase tracking-[0.18em] text-emerald-700">
              Confirmation
            </p>
            <h1 className="mt-3 text-3xl font-black tracking-tight text-emerald-950">
              Inscription enregistrée
            </h1>
            <p className="mx-auto mt-4 max-w-md text-sm leading-relaxed text-emerald-900/80">
              Un e-mail de confirmation vous a été envoyé avec un fichier calendrier (.ics) à
              ajouter à votre agenda.
            </p>
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[linear-gradient(180deg,#f5f3ff_0%,#f8fafc_42%,#ffffff_100%)]">
      <RentreePublicHeader />
      <main className="mx-auto w-full max-w-4xl px-4 py-8 sm:px-6 sm:py-12 lg:px-8">
        <div className="overflow-hidden rounded-[1.75rem] border border-violet-100/90 bg-white/95 shadow-[0_30px_80px_-48px_rgba(91,33,182,0.55)]">
          <header className="border-b border-violet-100 bg-[radial-gradient(120%_120%_at_0%_0%,#ede9fe_0%,#ffffff_55%)] px-5 py-7 sm:px-8 sm:py-9 lg:px-10">
            <p className="text-[11px] font-bold uppercase tracking-[0.2em] text-violet-700">
              Visite de l’établissement
            </p>
            <h1 className="mt-2 max-w-3xl text-3xl font-black tracking-tight text-violet-950 sm:text-4xl">
              {heading}
            </h1>
            {po.intro ? (
              <p className="mt-3 max-w-2xl text-sm leading-relaxed text-slate-600 sm:text-[15px]">
                {po.intro}
              </p>
            ) : null}
            {po.address ? (
              <p className="mt-5 inline-flex max-w-full flex-wrap items-center gap-x-2 gap-y-1 rounded-full bg-white/80 px-3.5 py-1.5 text-sm font-semibold text-slate-800 ring-1 ring-violet-100">
                <span aria-hidden>📍</span>
                <span>{po.address}</span>
                {po.mapsUrl ? (
                  <a
                    href={po.mapsUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="font-bold text-violet-700 underline-offset-2 hover:underline"
                  >
                    Itinéraire
                  </a>
                ) : null}
              </p>
            ) : null}
          </header>

          <div className="px-5 py-6 sm:px-8 sm:py-8 lg:px-10">
            {error ? (
              <p className="mb-5 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
                {error}
              </p>
            ) : null}

            {po.slots.length === 0 ? (
              <p className="text-sm text-slate-500">
                Les créneaux d&apos;inscription seront bientôt publiés.
              </p>
            ) : (
              <form onSubmit={(e) => void submit(e)} className="relative space-y-6">
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

                <section className="space-y-4">
                  <div className="flex flex-wrap items-end justify-between gap-3">
                    <div>
                      <h2 className="text-sm font-black uppercase tracking-wide text-violet-900">
                        Votre créneau
                      </h2>
                      <p className="mt-1 text-xs text-slate-500">
                        Choisissez l’établissement puis l’horaire de visite.
                      </p>
                    </div>
                    {dayLabel ? (
                      <p className="rounded-full bg-violet-50 px-3 py-1 text-xs font-bold text-violet-800 ring-1 ring-violet-100">
                        {dayLabel}
                      </p>
                    ) : null}
                  </div>

                  {availableCycles.length > 1 ? (
                    <div className="flex flex-wrap gap-2">
                      {availableCycles.map((c) => {
                        const active = form.cycle === c;
                        return (
                          <button
                            key={c}
                            type="button"
                            onClick={() => setCycle(c)}
                            className={`rounded-full px-4 py-2 text-sm font-bold transition ${
                              active
                                ? "bg-violet-700 text-white shadow-sm"
                                : "bg-slate-100 text-slate-700 hover:bg-violet-50 hover:text-violet-900"
                            }`}
                          >
                            {cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                          </button>
                        );
                      })}
                    </div>
                  ) : (
                    <p className="text-sm font-semibold text-slate-700">
                      {cycleLabels[form.cycle] || PORTES_OUVERTES_CYCLE_LABELS[form.cycle]}
                    </p>
                  )}

                  {cycleSlots.length === 0 ? (
                    <p className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                      Aucun créneau publié pour{" "}
                      {cycleLabels[form.cycle] || PORTES_OUVERTES_CYCLE_LABELS[form.cycle]}.
                    </p>
                  ) : allSlotsFull ? (
                    <p className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                      Tous les créneaux sont complets pour cet établissement.
                    </p>
                  ) : (
                    <div
                      role="listbox"
                      aria-label="Créneau horaire"
                      className="grid max-h-[min(24rem,58vh)] gap-2 overflow-y-auto sm:grid-cols-2 lg:grid-cols-3"
                    >
                      {cycleSlots.map((s) => {
                        const selected = form.slotId === s.id;
                        const disabled = s.remaining === 0;
                        const places = placesLabel(s.remaining);
                        return (
                          <button
                            key={s.id}
                            type="button"
                            role="option"
                            aria-selected={selected}
                            disabled={disabled}
                            onClick={() => setForm({ ...form, slotId: s.id })}
                            className={`flex min-h-[4.25rem] flex-col items-start justify-center rounded-2xl px-4 py-3 text-left transition ${
                              selected
                                ? "bg-violet-700 text-white shadow-md shadow-violet-700/25"
                                : disabled
                                  ? "cursor-not-allowed bg-slate-50 text-slate-400 ring-1 ring-slate-100"
                                  : "bg-slate-50 text-slate-900 ring-1 ring-slate-200/80 hover:bg-violet-50 hover:ring-violet-200"
                            }`}
                          >
                            <span className="text-base font-black tracking-tight sm:text-lg">
                              {formatHm(s.startAt)}
                              <span className={selected ? "text-violet-200" : "text-slate-400"}>
                                {" "}
                                – {formatHm(s.endAt)}
                              </span>
                            </span>
                            {places ? (
                              <span
                                className={`mt-1 text-xs font-semibold ${
                                  selected
                                    ? "text-violet-100"
                                    : disabled
                                      ? "text-slate-400"
                                      : "text-slate-500"
                                }`}
                              >
                                {places}
                              </span>
                            ) : null}
                          </button>
                        );
                      })}
                    </div>
                  )}
                  <input type="hidden" name="slotId" value={form.slotId} />
                </section>

                {!allSlotsFull && cycleSlots.length > 0 ? (
                  <div className="grid gap-6 lg:grid-cols-2 lg:gap-8">
                    <section className="space-y-4 rounded-[1.35rem] bg-slate-50/90 p-4 ring-1 ring-slate-100 sm:p-5">
                      <h2 className="text-sm font-black uppercase tracking-wide text-slate-700">
                        Vos coordonnées
                      </h2>
                      <div className="grid gap-4 sm:grid-cols-2">
                        <label className="block">
                          <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                            Prénom
                          </span>
                          <input
                            required
                            className={fieldClass}
                            value={form.firstName}
                            onChange={(e) => setForm({ ...form, firstName: e.target.value })}
                            autoComplete="given-name"
                          />
                        </label>
                        <label className="block">
                          <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                            Nom
                          </span>
                          <input
                            required
                            className={fieldClass}
                            value={form.lastName}
                            onChange={(e) => setForm({ ...form, lastName: e.target.value })}
                            autoComplete="family-name"
                          />
                        </label>
                      </div>
                      <label className="block">
                        <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                          E-mail
                        </span>
                        <input
                          required
                          type="email"
                          className={fieldClass}
                          value={form.email}
                          onChange={(e) => setForm({ ...form, email: e.target.value })}
                          autoComplete="email"
                        />
                      </label>
                      <label className="block">
                        <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                          Téléphone{" "}
                          <span className="font-medium normal-case text-slate-400">(optionnel)</span>
                        </span>
                        <input
                          className={fieldClass}
                          value={form.phone}
                          onChange={(e) => setForm({ ...form, phone: e.target.value })}
                          autoComplete="tel"
                        />
                      </label>
                    </section>

                    <section className="space-y-4 rounded-[1.35rem] bg-violet-50/50 p-4 ring-1 ring-violet-100 sm:p-5">
                      <h2 className="text-sm font-black uppercase tracking-wide text-violet-900">
                        Enfant concerné
                      </h2>
                      <div className="grid gap-4 sm:grid-cols-2">
                        <label className="block">
                          <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                            Prénom
                          </span>
                          <input
                            required
                            className={fieldClass}
                            value={form.childFirstName}
                            onChange={(e) => setForm({ ...form, childFirstName: e.target.value })}
                          />
                        </label>
                        <label className="block">
                          <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                            Nom
                          </span>
                          <input
                            required
                            className={fieldClass}
                            value={form.childLastName}
                            onChange={(e) => setForm({ ...form, childLastName: e.target.value })}
                          />
                        </label>
                      </div>
                      <label className="block">
                        <span className="text-xs font-bold uppercase tracking-wide text-slate-500">
                          Classe / niveau souhaité
                        </span>
                        <input
                          required
                          className={fieldClass}
                          value={form.classeSouhaitee}
                          onChange={(e) => setForm({ ...form, classeSouhaitee: e.target.value })}
                          placeholder="Ex. 6e, cinquième…"
                        />
                      </label>
                    </section>

                    <div className="lg:col-span-2">
                      <button
                        type="submit"
                        disabled={busy || selectedFull || !form.slotId}
                        className="w-full rounded-2xl bg-violet-700 px-4 py-3.5 text-sm font-black text-white shadow-lg shadow-violet-700/20 transition hover:bg-violet-800 disabled:opacity-50 sm:text-base"
                      >
                        {busy ? "Inscription…" : "Confirmer mon inscription"}
                      </button>
                    </div>
                  </div>
                ) : null}
              </form>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
