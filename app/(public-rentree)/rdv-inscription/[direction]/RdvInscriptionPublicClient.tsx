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

export type PublicRdvLevel = { id: string; label: string };

export type PublicRdvPageProps = {
  directionSlug: string;
  title: string;
  intro: string;
  consentLabel: string;
  location: string;
  directionLabel: string;
  directriceDisplayName: string | null;
  levels: PublicRdvLevel[];
  initialSlots: PublicRdvSlot[];
  initialError: string | null;
};

type MatchCandidate = {
  id: string;
  prenom: string;
  nom: string;
  classe: string | null;
  status: string;
};

type MatchChoice =
  | { kind: "eleve"; id: string; label: string }
  | { kind: "create" }
  | null;

function formatHm(iso: string): string {
  return new Date(iso).toLocaleTimeString("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDayChip(dayKey: string): { weekday: string; dayNum: string; month: string } {
  const d = parseParisDateTime(dayKey, "12:00");
  if (!d) return { weekday: "", dayNum: dayKey, month: "" };
  return {
    weekday: d.toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", weekday: "short" }),
    dayNum: d.toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", day: "numeric" }),
    month: d.toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", month: "short" }),
  };
}

function formatDayLong(dayKey: string): string {
  const d = parseParisDateTime(dayKey, "12:00");
  if (!d) return dayKey;
  return d.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
  });
}

function formatSlotRange(startAt: string, endAt: string): string {
  return `${new Date(startAt).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  })} – ${formatHm(endAt)}`;
}

const fieldClass =
  "mt-1.5 w-full rounded-lg border border-slate-200 bg-white px-3.5 py-2.5 text-slate-900 shadow-sm outline-none transition placeholder:text-slate-400 focus:border-sky-500 focus:ring-2 focus:ring-sky-100 disabled:bg-slate-50 disabled:text-slate-400";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function RdvInscriptionPublicClient({
  directionSlug,
  title,
  intro,
  consentLabel,
  location,
  directionLabel,
  directriceDisplayName,
  levels,
  initialSlots,
  initialError,
}: PublicRdvPageProps) {
  const [slots, setSlots] = useState(initialSlots);
  const [loadError, setLoadError] = useState(initialError);
  const [eventId, setEventId] = useState("");
  const [selectedDay, setSelectedDay] = useState<string>("");
  const [studentFirstName, setStudentFirstName] = useState("");
  const [studentLastName, setStudentLastName] = useState("");
  const [parentEmail, setParentEmail] = useState("");
  const [parentPhone, setParentPhone] = useState("");
  const [niveauId, setNiveauId] = useState(levels[0]?.id || "");
  const [candidates, setCandidates] = useState<MatchCandidate[] | null>(null);
  const [matchChoice, setMatchChoice] = useState<MatchChoice>(null);
  const [matchBusy, setMatchBusy] = useState(false);
  const [consent, setConsent] = useState(false);
  const [honeypot, setHoneypot] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<{
    pending?: boolean;
    startAt: string;
    endAt: string;
    mailWarning?: string;
  } | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const emailOk = EMAIL_RE.test(parentEmail.trim());
  const identityUnlocked = emailOk;

  const byDay = useMemo(() => {
    const map = new Map<string, PublicRdvSlot[]>();
    for (const s of slots) {
      const key = parisDateKey(s.startAt);
      const list = map.get(key) || [];
      list.push(s);
      map.set(key, list);
    }
    for (const list of map.values()) {
      list.sort((a, b) => a.startAt.localeCompare(b.startAt));
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [slots]);

  const dayKeys = useMemo(() => byDay.map(([k]) => k), [byDay]);

  useEffect(() => {
    if (!dayKeys.length) {
      setSelectedDay("");
      setEventId("");
      return;
    }
    setSelectedDay((prev) => (prev && dayKeys.includes(prev) ? prev : dayKeys[0]!));
  }, [dayKeys]);

  const daySlots = useMemo(() => {
    if (!selectedDay) return [];
    return byDay.find(([k]) => k === selectedDay)?.[1] || [];
  }, [byDay, selectedDay]);

  useEffect(() => {
    if (!daySlots.length) {
      setEventId("");
      return;
    }
    setEventId((prev) => (prev && daySlots.some((s) => s.eventId === prev) ? prev : ""));
  }, [daySlots]);

  useEffect(() => {
    if (!levels.length) return;
    if (!levels.some((l) => l.id === niveauId)) {
      setNiveauId(levels[0]!.id);
    }
  }, [levels, niveauId]);

  const selectedSlot = useMemo(
    () => slots.find((s) => s.eventId === eventId) || null,
    [slots, eventId],
  );

  const matchReady = Boolean(matchChoice);

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
    } catch {
      setLoadError("Impossible de charger les créneaux.");
    }
  }, [directionSlug]);

  useEffect(() => {
    if (!initialError && initialSlots.length === 0) {
      void refreshSlots();
    }
  }, [initialError, initialSlots.length, refreshSlots]);

  function resetMatch() {
    setCandidates(null);
    setMatchChoice(null);
  }

  async function onSearchChild() {
    setFormError(null);
    if (!emailOk) {
      setFormError("Saisissez d’abord un e-mail parent valide.");
      return;
    }
    if (!studentFirstName.trim() || !studentLastName.trim()) {
      setFormError("Indiquez le nom et le prénom de l’élève.");
      return;
    }
    setMatchBusy(true);
    resetMatch();
    try {
      const res = await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/match`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          parentEmail,
          parentPhone,
          studentFirstName,
          studentLastName,
        }),
      });
      const data = (await res.json()) as {
        candidates?: MatchCandidate[];
        error?: string;
      };
      if (!res.ok) {
        setFormError(data.error || "Recherche impossible.");
        return;
      }
      setCandidates(data.candidates || []);
    } catch {
      setFormError("Erreur réseau — réessayez.");
    } finally {
      setMatchBusy(false);
    }
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setFormError(null);
    if (!matchChoice) {
      setFormError("Confirmez l’élève (ou créez un nouveau dossier) avant de réserver.");
      return;
    }
    if (!niveauId) {
      setFormError("Choisissez le niveau demandé.");
      return;
    }
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
          niveauId,
          eleveId: matchChoice.kind === "eleve" ? matchChoice.id : null,
          createNew: matchChoice.kind === "create",
          consent: true,
          website: honeypot,
        }),
      });
      const data = (await res.json()) as {
        success?: boolean;
        pending?: boolean;
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
        pending: data.pending === true,
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
    const isPending = done.pending === true;
    return (
      <div className="min-h-screen bg-[#f7f8fa]">
        <RentreePublicHeader />
        <main className="mx-auto max-w-lg px-4 py-14 text-center">
          <div
            className={`mx-auto flex h-14 w-14 items-center justify-center rounded-full text-2xl ${
              isPending ? "bg-sky-100 text-sky-700" : "bg-emerald-100 text-emerald-700"
            }`}
          >
            {isPending ? "✉" : "✓"}
          </div>
          <h1 className="mt-5 text-2xl font-bold tracking-tight text-slate-900">
            {isPending ? "Vérifiez votre e-mail" : "Rendez-vous confirmé"}
          </h1>
          <p className="mt-2 text-slate-600">
            {directionLabel}
            {directriceDisplayName ? ` · ${directriceDisplayName}` : ""}
          </p>
          {done.startAt ? (
            <p className="mt-4 rounded-xl bg-white px-4 py-3 text-sm font-medium text-slate-800 shadow-sm ring-1 ring-slate-200/80">
              {formatSlotRange(done.startAt, done.endAt || done.startAt)}
            </p>
          ) : null}
          <p className="mt-5 text-sm leading-relaxed text-slate-500">
            {isPending
              ? "Un e-mail vient de vous être envoyé. Cliquez sur le lien pour valider votre créneau (valable 2 heures). Sans validation, le créneau sera libéré."
              : "Un e-mail de confirmation avec fichier calendrier (.ics) vous a été envoyé."}
          </p>
          {done.mailWarning ? (
            <p className="mt-3 text-sm text-amber-800">{done.mailWarning}</p>
          ) : null}
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#f7f8fa]">
      <RentreePublicHeader />
      <main className="mx-auto max-w-xl px-4 py-8 sm:py-10">
        <header className="mb-8">
          <p className="text-xs font-semibold uppercase tracking-[0.14em] text-sky-700">
            {directionLabel}
            {directriceDisplayName ? ` · ${directriceDisplayName}` : ""}
          </p>
          <h1 className="mt-2 text-3xl font-bold tracking-tight text-slate-900 sm:text-[2rem]">
            {title}
          </h1>
          {intro ? (
            <p className="mt-3 text-[15px] leading-relaxed text-slate-600 whitespace-pre-line">
              {intro}
            </p>
          ) : null}
          {location ? (
            <p className="mt-3 inline-flex items-center gap-1.5 rounded-full bg-white px-3 py-1 text-sm text-slate-600 ring-1 ring-slate-200/80">
              <span className="text-slate-400">Lieu</span>
              <span className="font-medium text-slate-800">{location}</span>
            </p>
          ) : null}
        </header>

        {loadError ? (
          <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
            {loadError}
          </div>
        ) : null}

        {!loadError && slots.length === 0 ? (
          <div className="rounded-xl bg-white px-5 py-8 text-center text-slate-600 shadow-sm ring-1 ring-slate-200/80">
            Aucun créneau disponible pour le moment.
            <br />
            <span className="text-sm text-slate-500">
              Revenez plus tard ou contactez l’établissement.
            </span>
          </div>
        ) : null}

        {slots.length > 0 ? (
          <form onSubmit={onSubmit} className="space-y-6">
            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                1 · Vos coordonnées
              </h2>
              <p className="mt-2 text-sm text-slate-500">
                L’e-mail est demandé en premier pour protéger les dossiers élèves.
              </p>
              <div className="mt-4 grid gap-4">
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">E-mail du parent</span>
                  <input
                    required
                    type="email"
                    className={fieldClass}
                    value={parentEmail}
                    onChange={(e) => {
                      setParentEmail(e.target.value);
                      resetMatch();
                    }}
                    autoComplete="email"
                    placeholder="prenom.nom@email.fr"
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">Téléphone du parent</span>
                  <input
                    required
                    type="tel"
                    className={fieldClass}
                    value={parentPhone}
                    onChange={(e) => {
                      setParentPhone(e.target.value);
                      resetMatch();
                    }}
                    autoComplete="tel"
                    placeholder="06 …"
                    disabled={!identityUnlocked}
                  />
                </label>
              </div>
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                2 · Identifier l’élève
              </h2>
              <div className="mt-4 grid gap-4 sm:grid-cols-2">
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">Prénom de l’élève</span>
                  <input
                    required
                    className={fieldClass}
                    value={studentFirstName}
                    onChange={(e) => {
                      setStudentFirstName(e.target.value);
                      resetMatch();
                    }}
                    autoComplete="given-name"
                    placeholder="Prénom"
                    disabled={!identityUnlocked}
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">Nom de l’élève</span>
                  <input
                    required
                    className={fieldClass}
                    value={studentLastName}
                    onChange={(e) => {
                      setStudentLastName(e.target.value);
                      resetMatch();
                    }}
                    autoComplete="family-name"
                    placeholder="Nom"
                    disabled={!identityUnlocked}
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={!identityUnlocked || matchBusy}
                onClick={() => void onSearchChild()}
                className="mt-4 w-full rounded-xl bg-slate-800 px-4 py-3 text-sm font-bold text-white transition hover:bg-slate-900 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {matchBusy ? "Recherche…" : "Rechercher mon enfant"}
              </button>

              {candidates ? (
                <div className="mt-4 space-y-2">
                  {candidates.length === 0 ? (
                    <p className="rounded-lg bg-slate-50 px-3 py-2 text-sm text-slate-600">
                      Aucun enfant trouvé pour ces coordonnées. Vous pouvez créer un nouveau
                      dossier.
                    </p>
                  ) : (
                    candidates.map((c) => {
                      const selected =
                        matchChoice?.kind === "eleve" && matchChoice.id === c.id;
                      const label = `${c.nom.toUpperCase()} ${c.prenom}${
                        c.classe ? ` · ${c.classe}` : ""
                      }`;
                      return (
                        <button
                          key={c.id}
                          type="button"
                          onClick={() =>
                            setMatchChoice({ kind: "eleve", id: c.id, label })
                          }
                          className={`w-full rounded-xl px-4 py-3 text-left text-sm transition ${
                            selected
                              ? "bg-sky-700 text-white shadow-sm"
                              : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white"
                          }`}
                        >
                          <span className="font-semibold">
                            Est-ce bien {c.prenom} {c.nom.toUpperCase()} ?
                          </span>
                          {c.classe ? (
                            <span
                              className={`mt-0.5 block text-xs ${
                                selected ? "text-sky-100" : "text-slate-500"
                              }`}
                            >
                              {c.classe}
                            </span>
                          ) : null}
                        </button>
                      );
                    })
                  )}
                  <button
                    type="button"
                    onClick={() => setMatchChoice({ kind: "create" })}
                    className={`w-full rounded-xl px-4 py-3 text-left text-sm font-semibold transition ${
                      matchChoice?.kind === "create"
                        ? "bg-amber-600 text-white"
                        : "bg-amber-50 text-amber-950 ring-1 ring-amber-200 hover:bg-amber-100"
                    }`}
                  >
                    Créer un nouveau dossier préinscrit
                  </button>
                  {matchChoice?.kind === "eleve" ? (
                    <p className="text-sm text-emerald-700">
                      Élève confirmé : <strong>{matchChoice.label}</strong>
                    </p>
                  ) : null}
                  {matchChoice?.kind === "create" ? (
                    <p className="text-sm text-amber-800">
                      Un nouveau dossier sera créé à la validation du rendez-vous.
                    </p>
                  ) : null}
                </div>
              ) : null}
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                3 · Niveau demandé
              </h2>
              <label className="mt-4 block text-sm">
                <span className="font-semibold text-slate-800">Classe / formation</span>
                <select
                  required
                  className={fieldClass}
                  value={niveauId}
                  onChange={(e) => setNiveauId(e.target.value)}
                  disabled={!matchReady}
                >
                  {levels.map((l) => (
                    <option key={l.id} value={l.id}>
                      {l.label}
                    </option>
                  ))}
                </select>
              </label>
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <div className="flex flex-wrap items-end justify-between gap-2">
                <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                  4 · Choisir un créneau
                </h2>
                <p className="text-xs text-slate-500">
                  {slots.length} créneau{slots.length > 1 ? "x" : ""} · {dayKeys.length} jour
                  {dayKeys.length > 1 ? "s" : ""}
                </p>
              </div>

              {!matchReady ? (
                <p className="mt-4 text-sm text-slate-500">
                  Confirmez d’abord l’élève pour débloquer les créneaux.
                </p>
              ) : (
                <>
                  <p className="mt-4 text-sm font-semibold text-slate-800">Jour</p>
                  <div
                    className="-mx-1 mt-2 flex gap-2 overflow-x-auto px-1 pb-1 [scrollbar-width:thin]"
                    role="listbox"
                    aria-label="Choisir un jour"
                  >
                    {byDay.map(([day, dayList]) => {
                      const chip = formatDayChip(day);
                      const active = day === selectedDay;
                      return (
                        <button
                          key={day}
                          type="button"
                          role="option"
                          aria-selected={active}
                          onClick={() => setSelectedDay(day)}
                          className={`flex min-w-[4.5rem] shrink-0 flex-col items-center rounded-xl px-3 py-2.5 text-center transition ${
                            active
                              ? "bg-sky-700 text-white shadow-sm"
                              : "bg-slate-50 text-slate-700 ring-1 ring-slate-200 hover:bg-slate-100"
                          }`}
                        >
                          <span
                            className={`text-[11px] font-semibold uppercase tracking-wide ${
                              active ? "text-sky-100" : "text-slate-500"
                            }`}
                          >
                            {chip.weekday}
                          </span>
                          <span className="mt-0.5 text-xl font-bold leading-none">
                            {chip.dayNum}
                          </span>
                          <span
                            className={`mt-1 text-[11px] font-medium capitalize ${
                              active ? "text-sky-100" : "text-slate-500"
                            }`}
                          >
                            {chip.month}
                          </span>
                          <span
                            className={`mt-1.5 text-[10px] ${
                              active ? "text-sky-200" : "text-slate-400"
                            }`}
                          >
                            {dayList.length} crén.
                          </span>
                        </button>
                      );
                    })}
                  </div>

                  {selectedDay ? (
                    <>
                      <p className="mt-5 text-sm font-semibold text-slate-800">
                        Horaires — {formatDayLong(selectedDay)}
                      </p>
                      <div className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3">
                        {daySlots.map((s) => {
                          const selected = s.eventId === eventId;
                          return (
                            <button
                              key={s.eventId}
                              type="button"
                              onClick={() => setEventId(s.eventId)}
                              className={`rounded-xl px-3 py-3 text-center text-sm font-semibold transition ${
                                selected
                                  ? "bg-sky-700 text-white shadow-sm ring-2 ring-sky-700 ring-offset-2"
                                  : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white hover:ring-sky-300"
                              }`}
                            >
                              <span className="block text-base tabular-nums">
                                {formatHm(s.startAt)}
                              </span>
                              <span
                                className={`mt-0.5 block text-xs font-medium tabular-nums ${
                                  selected ? "text-sky-100" : "text-slate-500"
                                }`}
                              >
                                → {formatHm(s.endAt)}
                              </span>
                            </button>
                          );
                        })}
                      </div>
                    </>
                  ) : null}

                  {selectedSlot ? (
                    <p className="mt-4 rounded-lg bg-sky-50 px-3 py-2 text-sm text-sky-950">
                      Sélection :{" "}
                      <strong>
                        {formatSlotRange(selectedSlot.startAt, selectedSlot.endAt)}
                      </strong>
                    </p>
                  ) : (
                    <p className="mt-4 text-sm text-slate-500">
                      Sélectionnez un jour puis un horaire.
                    </p>
                  )}
                </>
              )}
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <label className="flex items-start gap-3 text-sm leading-relaxed text-slate-700">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4 rounded border-slate-300 text-sky-700 focus:ring-sky-500"
                  checked={consent}
                  onChange={(e) => setConsent(e.target.checked)}
                />
                <span>{consentLabel}</span>
              </label>

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
                <p className="mt-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
                  {formError}
                </p>
              ) : null}

              <button
                type="submit"
                disabled={busy || !matchReady || !eventId}
                className="mt-5 w-full rounded-xl bg-sky-700 px-4 py-3.5 text-sm font-bold text-white shadow-sm transition hover:bg-sky-800 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {busy ? "Réservation…" : "Confirmer le rendez-vous"}
              </button>
            </section>
          </form>
        ) : null}
      </main>
    </div>
  );
}
