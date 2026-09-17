"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import { parisDateKey, parseParisDateTime } from "@/app/lib/paris-time";

export type PublicRdvSlot = {
  eventId: string;
  startAt: string;
  endAt: string;
  title: string;
};

export type PublicRdvPageProps = {
  directionSlug: string;
  title: string;
  intro: string;
  consentLabel: string;
  location: string;
  directionLabel: string;
  directriceDisplayName: string | null;
  initialSlots: PublicRdvSlot[];
  initialError: string | null;
};

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

export default function RdvInscriptionPublicClient({
  directionSlug,
  title,
  intro,
  consentLabel,
  location,
  directionLabel,
  directriceDisplayName,
  initialSlots,
  initialError,
}: PublicRdvPageProps) {
  const [slots, setSlots] = useState(initialSlots);
  const [loadError, setLoadError] = useState(initialError);
  const [eventId, setEventId] = useState(initialSlots[0]?.eventId || "");
  const [studentFirstName, setStudentFirstName] = useState("");
  const [studentLastName, setStudentLastName] = useState("");
  const [parentEmail, setParentEmail] = useState("");
  const [parentPhone, setParentPhone] = useState("");
  const [consent, setConsent] = useState(false);
  const [honeypot, setHoneypot] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<{ startAt: string; endAt: string; mailWarning?: string } | null>(
    null,
  );
  const [formError, setFormError] = useState<string | null>(null);

  const refreshSlots = useCallback(async () => {
    try {
      const res = await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/slots`);
      const data = (await res.json()) as { slots?: PublicRdvSlot[]; error?: string };
      if (!res.ok) {
        setLoadError(data.error || "Impossible de charger les créneaux.");
        setSlots([]);
        return;
      }
      setLoadError(null);
      setSlots(data.slots || []);
      if (data.slots?.length && !data.slots.some((s) => s.eventId === eventId)) {
        setEventId(data.slots[0]!.eventId);
      }
    } catch {
      setLoadError("Impossible de charger les créneaux.");
    }
  }, [directionSlug, eventId]);

  useEffect(() => {
    if (!initialError && initialSlots.length === 0) {
      void refreshSlots();
    }
  }, [initialError, initialSlots.length, refreshSlots]);

  const byDay = useMemo(() => {
    const map = new Map<string, PublicRdvSlot[]>();
    for (const s of slots) {
      const key = parisDateKey(s.startAt);
      const list = map.get(key) || [];
      list.push(s);
      map.set(key, list);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [slots]);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setFormError(null);
    if (!eventId) {
      setFormError("Choisissez un créneau.");
      return;
    }
    if (!consent) {
      setFormError("Merci d’accepter le traitement de vos coordonnées.");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/book`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eventId,
          studentFirstName,
          studentLastName,
          parentEmail,
          parentPhone,
          consent: true,
          website: honeypot,
        }),
      });
      const data = (await res.json()) as {
        success?: boolean;
        error?: string;
        startAt?: string;
        endAt?: string;
        mailWarning?: string;
      };
      if (!res.ok || !data.success) {
        setFormError(data.error || "Réservation impossible.");
        if (res.status === 409 || res.status === 410) {
          await refreshSlots();
        }
        return;
      }
      setDone({
        startAt: data.startAt || "",
        endAt: data.endAt || "",
        mailWarning: data.mailWarning,
      });
    } catch {
      setFormError("Erreur réseau — réessayez.");
    } finally {
      setBusy(false);
    }
  }

  if (done) {
    return (
      <div className="min-h-screen bg-gradient-to-b from-slate-50 to-white">
        <RentreePublicHeader />
        <main className="mx-auto max-w-lg px-4 py-12 text-center">
          <h1 className="text-2xl font-bold text-slate-900">Rendez-vous confirmé</h1>
          <p className="mt-3 text-slate-600">
            {directionLabel}
            {done.startAt
              ? ` — ${new Date(done.startAt).toLocaleString("fr-FR", { timeZone: "Europe/Paris" })}`
              : ""}
          </p>
          <p className="mt-4 text-sm text-slate-500">
            Un e-mail de confirmation avec fichier calendrier (.ics) vous a été envoyé
            {done.mailWarning ? " (si la messagerie de l’établissement est opérationnelle)" : ""}.
          </p>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-50 to-white">
      <RentreePublicHeader />
      <main className="mx-auto max-w-xl px-4 py-8 sm:py-12">
        <p className="text-sm font-semibold uppercase tracking-wide text-sky-700">
          {directionLabel}
          {directriceDisplayName ? ` · ${directriceDisplayName}` : ""}
        </p>
        <h1 className="mt-1 text-3xl font-bold text-slate-900">{title}</h1>
        {intro ? <p className="mt-3 text-slate-600 whitespace-pre-line">{intro}</p> : null}
        {location ? (
          <p className="mt-2 text-sm text-slate-500">
            Lieu : <span className="font-medium text-slate-700">{location}</span>
          </p>
        ) : null}

        {loadError ? (
          <div className="mt-8 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-amber-900">
            {loadError}
          </div>
        ) : null}

        {!loadError && slots.length === 0 ? (
          <div className="mt-8 rounded-lg border border-slate-200 bg-white px-4 py-6 text-slate-600">
            Aucun créneau disponible pour le moment. Revenez plus tard ou contactez l’établissement.
          </div>
        ) : null}

        {slots.length > 0 ? (
          <form onSubmit={onSubmit} className="mt-8 space-y-6">
            <fieldset>
              <legend className="text-sm font-bold text-slate-800">Créneau</legend>
              <div className="mt-3 space-y-4">
                {byDay.map(([day, daySlots]) => (
                  <div key={day}>
                    <p className="text-xs font-semibold uppercase text-slate-500">
                      {formatDayLong(day)}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-2">
                      {daySlots.map((s) => {
                        const selected = s.eventId === eventId;
                        return (
                          <button
                            key={s.eventId}
                            type="button"
                            onClick={() => setEventId(s.eventId)}
                            className={`rounded-md border px-3 py-2 text-sm font-semibold transition ${
                              selected
                                ? "border-sky-600 bg-sky-600 text-white"
                                : "border-slate-200 bg-white text-slate-800 hover:border-sky-300"
                            }`}
                          >
                            {formatHm(s.startAt)} – {formatHm(s.endAt)}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </fieldset>

            <div className="grid gap-4 sm:grid-cols-2">
              <label className="block text-sm">
                <span className="font-semibold text-slate-700">Prénom de l’élève</span>
                <input
                  required
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                  value={studentFirstName}
                  onChange={(e) => setStudentFirstName(e.target.value)}
                  autoComplete="given-name"
                />
              </label>
              <label className="block text-sm">
                <span className="font-semibold text-slate-700">Nom de l’élève</span>
                <input
                  required
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                  value={studentLastName}
                  onChange={(e) => setStudentLastName(e.target.value)}
                  autoComplete="family-name"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold text-slate-700">E-mail du parent</span>
                <input
                  required
                  type="email"
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                  value={parentEmail}
                  onChange={(e) => setParentEmail(e.target.value)}
                  autoComplete="email"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold text-slate-700">Téléphone du parent</span>
                <input
                  required
                  type="tel"
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                  value={parentPhone}
                  onChange={(e) => setParentPhone(e.target.value)}
                  autoComplete="tel"
                />
              </label>
            </div>

            <label className="flex items-start gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                className="mt-1"
                checked={consent}
                onChange={(e) => setConsent(e.target.checked)}
              />
              <span>{consentLabel}</span>
            </label>

            {/* Honeypot anti-bot */}
            <input
              type="text"
              name="website"
              value={honeypot}
              onChange={(e) => setHoneypot(e.target.value)}
              className="absolute -left-[9999px] h-0 w-0 opacity-0"
              tabIndex={-1}
              autoComplete="off"
              aria-hidden
            />

            {formError ? (
              <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
                {formError}
              </p>
            ) : null}

            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-md bg-sky-700 px-4 py-3 text-sm font-bold text-white hover:bg-sky-800 disabled:opacity-60"
            >
              {busy ? "Réservation…" : "Confirmer le rendez-vous"}
            </button>
          </form>
        ) : null}
      </main>
    </div>
  );
}
