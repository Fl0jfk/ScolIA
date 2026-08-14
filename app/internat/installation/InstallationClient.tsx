"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { InternatInstallationPublicSlot } from "@/app/lib/internat-types";

type PublicPayload = {
  enabled: boolean;
  title: string;
  intro: string | null;
  location: string | null;
  slots: InternatInstallationPublicSlot[];
  error?: string;
};

function dateHeading(date: string): string {
  return new Date(`${date}T12:00:00`).toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
}

export default function InstallationClient() {
  const [data, setData] = useState<PublicPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [slotStart, setSlotStart] = useState("");
  const [studentFirstName, setStudentFirstName] = useState("");
  const [studentLastName, setStudentLastName] = useState("");
  const [parentPhone, setParentPhone] = useState("");
  const [parentEmail, setParentEmail] = useState("");
  const [honeypot, setHoneypot] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [doneLabel, setDoneLabel] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/internat/installation/public", { cache: "no-store" });
      const json = (await res.json()) as PublicPayload;
      if (!res.ok) throw new Error(json.error || "Chargement impossible.");
      setData(json);
      if (json.slots.length && !json.slots.some((s) => s.slotStart === slotStart)) {
        setSlotStart(json.slots[0].slotStart);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [slotStart]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- charge initiale une fois
  }, []);

  const slotsByDay = useMemo(() => {
    const map = new Map<string, InternatInstallationPublicSlot[]>();
    for (const s of data?.slots ?? []) {
      const list = map.get(s.date) ?? [];
      list.push(s);
      map.set(s.date, list);
    }
    return [...map.entries()];
  }, [data?.slots]);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!slotStart) {
      setError("Choisissez un créneau.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/internat/installation/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          slotStart,
          studentFirstName,
          studentLastName,
          parentPhone,
          parentEmail,
          website: honeypot,
        }),
      });
      const json = (await res.json()) as { error?: string; slotLabel?: string };
      if (!res.ok) throw new Error(json.error || "Inscription impossible.");
      setDoneLabel(json.slotLabel || "votre créneau");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
      void load();
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return (
      <div className="mx-auto max-w-lg px-4 py-16 text-center text-slate-500 text-sm">
        Chargement des créneaux…
      </div>
    );
  }

  if (doneLabel) {
    return (
      <div className="mx-auto max-w-lg px-4 py-16 text-center space-y-3">
        <h1 className="text-2xl font-black text-slate-900">Rendez-vous confirmé</h1>
        <p className="text-slate-600 text-sm">
          Créneau : <strong>{doneLabel}</strong>
        </p>
        <p className="text-slate-500 text-sm">
          Un e-mail de confirmation avec un fichier calendrier (.ics) vous a été envoyé.
        </p>
      </div>
    );
  }

  if (!data?.enabled) {
    return (
      <div className="mx-auto max-w-lg px-4 py-16 text-center space-y-2">
        <h1 className="text-2xl font-black text-slate-900">
          {data?.title || "Installation internat"}
        </h1>
        <p className="text-slate-500 text-sm">
          Les prises de rendez-vous ne sont pas ouvertes pour le moment.
        </p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-lg px-4 py-10 sm:py-14">
      <header className="mb-8 space-y-2">
        <h1 className="text-2xl sm:text-3xl font-black text-slate-900 tracking-tight">
          {data.title}
        </h1>
        {data.intro ? <p className="text-sm text-slate-600 leading-relaxed">{data.intro}</p> : null}
        {data.location ? (
          <p className="text-sm text-slate-500">
            Lieu : <span className="font-medium text-slate-700">{data.location}</span>
          </p>
        ) : null}
      </header>

      {error ? (
        <p className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700">
          {error}
        </p>
      ) : null}

      {!data.slots.length ? (
        <p className="text-sm text-slate-500">Aucun créneau disponible pour le moment.</p>
      ) : (
        <form onSubmit={submit} className="relative space-y-6">
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
          <fieldset className="space-y-4">
            <legend className="text-sm font-bold text-slate-900">Choisissez un créneau</legend>
            {slotsByDay.map(([date, slots]) => (
              <div key={date} className="space-y-2">
                <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
                  {dateHeading(date)}
                </p>
                <div className="flex flex-wrap gap-2">
                  {slots.map((s) => {
                    const selected = slotStart === s.slotStart;
                    return (
                      <button
                        key={s.slotStart}
                        type="button"
                        onClick={() => setSlotStart(s.slotStart)}
                        className={`rounded-full border px-3 py-1.5 text-sm font-medium transition ${
                          selected
                            ? "border-indigo-600 bg-indigo-600 text-white"
                            : "border-slate-200 bg-white text-slate-700 hover:border-indigo-300"
                        }`}
                      >
                        {s.time}
                        {s.capacity > 1 ? (
                          <span className="ml-1 opacity-70">({s.remaining})</span>
                        ) : null}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </fieldset>

          <div className="space-y-3 rounded-2xl border border-slate-200 bg-white p-4">
            <p className="text-sm font-bold text-slate-900">Coordonnées</p>
            <label className="block space-y-1 text-sm">
              <span className="text-slate-600">Prénom de l’élève</span>
              <input
                required
                value={studentFirstName}
                onChange={(e) => setStudentFirstName(e.target.value)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:border-indigo-400"
              />
            </label>
            <label className="block space-y-1 text-sm">
              <span className="text-slate-600">Nom de l’élève</span>
              <input
                required
                value={studentLastName}
                onChange={(e) => setStudentLastName(e.target.value)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:border-indigo-400"
              />
            </label>
            <label className="block space-y-1 text-sm">
              <span className="text-slate-600">Téléphone du parent</span>
              <input
                required
                type="tel"
                value={parentPhone}
                onChange={(e) => setParentPhone(e.target.value)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:border-indigo-400"
              />
            </label>
            <label className="block space-y-1 text-sm">
              <span className="text-slate-600">E-mail du parent</span>
              <input
                required
                type="email"
                value={parentEmail}
                onChange={(e) => setParentEmail(e.target.value)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:border-indigo-400"
              />
            </label>
          </div>

          <button
            type="submit"
            disabled={busy || !slotStart}
            className="w-full rounded-full bg-indigo-700 px-4 py-3 text-sm font-semibold text-white hover:bg-indigo-800 disabled:opacity-50"
          >
            {busy ? "Inscription…" : "Confirmer le rendez-vous"}
          </button>
        </form>
      )}
    </div>
  );
}
