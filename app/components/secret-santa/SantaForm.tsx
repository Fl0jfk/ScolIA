"use client";

import { useEffect, useMemo, useState } from "react";
import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";
import { SANTA_MIN_PARTICIPANTS, santaMaxExclusionsPerPerson, type SantaExclusion } from "@/app/lib/secret-santa";

type Member = { userId: string; displayName: string; roles: string[] };

export type SantaFormValues = {
  title: string;
  budget: string;
  message: string;
  /** ISO. */
  revealAt: string;
  participantIds: string[];
  exclusions: SantaExclusion[];
};

const input = "mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm";
const label = "text-xs font-bold uppercase text-slate-500";
const norm = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

/** Valeur d'un champ `datetime-local` (heure du poste). */
function toLocalInput(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export default function SantaForm({
  initial,
  submitLabel,
  onSubmit,
  onCancel,
}: {
  initial?: SantaFormValues;
  submitLabel: string;
  onSubmit: (values: SantaFormValues) => Promise<void>;
  onCancel: () => void;
}) {
  const [staff, setStaff] = useState<Member[] | null>(null);
  const [title, setTitle] = useState(initial?.title ?? "");
  const [budget, setBudget] = useState(initial?.budget ?? "");
  const [message, setMessage] = useState(initial?.message ?? "");
  const [revealAt, setRevealAt] = useState(toLocalInput(initial?.revealAt ?? ""));
  const [selected, setSelected] = useState<Set<string>>(new Set(initial?.participantIds ?? []));
  const [exclusions, setExclusions] = useState<SantaExclusion[]>(initial?.exclusions ?? []);
  const [pair, setPair] = useState<[string, string]>(["", ""]);
  const [search, setSearch] = useState("");
  const [role, setRole] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const res = await fetch("/api/secret-santa/annuaire", { cache: "no-store" });
        const data = await res.json();
        if (!res.ok) throw new Error(data?.error || "Annuaire indisponible.");
        if (!alive) return;
        setStaff(data.staff);
        // Nouvel événement : l'organisateur participe par défaut (case décochable).
        if (!initial) setSelected(new Set([data.me]));
      } catch (e) {
        if (alive) setError(e instanceof Error ? e.message : "Annuaire indisponible.");
      }
    })();
    return () => {
      alive = false;
    };
    // Chargé une seule fois à l'ouverture du formulaire.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const names = useMemo(() => new Map((staff ?? []).map((m) => [m.userId, m.displayName])), [staff]);
  const visible = useMemo(
    () =>
      (staff ?? []).filter(
        (m) => (!role || m.roles.includes(role)) && norm(m.displayName).includes(norm(search.trim())),
      ),
    [staff, role, search],
  );
  const roleOptions = INTRANET_ROLE_OPTIONS.filter((o) => (staff ?? []).some((m) => m.roles.includes(o.slug)));
  const chosen = (staff ?? []).filter((m) => selected.has(m.userId));
  const missing = [...selected].filter((id) => staff && !names.has(id));

  const setMany = (ids: string[], on: boolean) =>
    setSelected((previous) => {
      const next = new Set(previous);
      for (const id of ids) {
        if (on) next.add(id);
        else next.delete(id);
      }
      return next;
    });

  const addExclusion = () => {
    const [a, b] = pair;
    if (!a || !b || a === b) return;
    if (!exclusions.some(([x, y]) => (x === a && y === b) || (x === b && y === a))) {
      setExclusions([...exclusions, [a, b]]);
    }
    setPair(["", ""]);
  };

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    const date = new Date(revealAt);
    if (Number.isNaN(date.getTime())) {
      setError("Choisissez la date et l'heure du tirage.");
      return;
    }
    setBusy(true);
    try {
      const participantIds = chosen.map((m) => m.userId);
      await onSubmit({
        title,
        budget,
        message,
        revealAt: date.toISOString(),
        participantIds,
        exclusions: exclusions.filter(([a, b]) => selected.has(a) && selected.has(b)),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Enregistrement impossible.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="space-y-5 rounded-2xl border border-slate-200 bg-white p-5">
      <div className="grid gap-4 sm:grid-cols-2">
        <label className="block sm:col-span-2">
          <span className={label}>Titre</span>
          <input
            className={input}
            value={title}
            maxLength={120}
            required
            placeholder="Secret Santa des professeurs"
            onChange={(e) => setTitle(e.target.value)}
          />
        </label>
        <label className="block">
          <span className={label}>Date et heure du tirage</span>
          <input
            type="datetime-local"
            className={input}
            value={revealAt}
            required
            onChange={(e) => setRevealAt(e.target.value)}
          />
        </label>
        <label className="block">
          <span className={label}>Budget (facultatif)</span>
          <input
            className={input}
            value={budget}
            maxLength={80}
            placeholder="Environ 15 €"
            onChange={(e) => setBudget(e.target.value)}
          />
        </label>
        <label className="block sm:col-span-2">
          <span className={label}>Message aux participants (facultatif)</span>
          <textarea
            className={`${input} min-h-[70px]`}
            value={message}
            maxLength={1000}
            placeholder="Échange des cadeaux le 18 décembre en salle des professeurs."
            onChange={(e) => setMessage(e.target.value)}
          />
        </label>
      </div>

      <fieldset>
        <legend className={label}>Participants ({chosen.length})</legend>
        <div className="mt-2 flex flex-wrap gap-2">
          <input
            className="min-w-0 flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
            value={search}
            placeholder="Rechercher un nom…"
            aria-label="Rechercher un nom"
            onChange={(e) => setSearch(e.target.value)}
          />
          <select
            className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
            value={role}
            aria-label="Filtrer par rôle"
            onChange={(e) => setRole(e.target.value)}
          >
            <option value="">Tout le personnel</option>
            {roleOptions.map((o) => (
              <option key={o.slug} value={o.slug}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
        <div className="mt-2 flex flex-wrap gap-3 text-xs font-bold">
          <button
            type="button"
            className="text-red-700 underline"
            onClick={() => setMany(visible.map((m) => m.userId), true)}
          >
            Cocher les {visible.length} affiché(s)
          </button>
          <button
            type="button"
            className="text-slate-500 underline"
            onClick={() => setMany(visible.map((m) => m.userId), false)}
          >
            Les décocher
          </button>
        </div>
        <ul className="mt-2 grid max-h-64 gap-1 overflow-y-auto rounded-xl border border-slate-200 p-2 sm:grid-cols-2">
          {!staff && <li className="p-2 text-sm text-slate-400">Chargement de l&apos;annuaire…</li>}
          {visible.map((m) => (
            <li key={m.userId}>
              <label className="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-sm hover:bg-slate-50">
                <input
                  type="checkbox"
                  checked={selected.has(m.userId)}
                  onChange={(e) => setMany([m.userId], e.target.checked)}
                />
                {m.displayName}
              </label>
            </li>
          ))}
        </ul>
        <p className="mt-1 text-xs text-slate-500">
          Seules les personnes qui ont un compte ScolIA apparaissent. Vous participez si votre nom est coché.
        </p>
        {missing.length > 0 && (
          <p className="mt-1 text-xs text-amber-700">
            {missing.length} personne(s) ne sont plus dans l&apos;annuaire et seront retirées.
          </p>
        )}
      </fieldset>

      <fieldset>
        <legend className={label}>Paires interdites (facultatif)</legend>
        <p className="mt-1 text-xs text-slate-500">
          Par exemple un couple : ces deux personnes ne pourront pas se tirer, dans un sens comme dans
          l&apos;autre. Avec {chosen.length} participants : {Math.max(0, santaMaxExclusionsPerPerson(chosen.length))}{" "}
          au maximum par personne, pour que le tirage reste secret.
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          {[0, 1].map((side) => (
            <select
              key={side}
              className="min-w-0 flex-1 rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
              value={pair[side]}
              aria-label={side === 0 ? "Première personne" : "Deuxième personne"}
              onChange={(e) => setPair(side === 0 ? [e.target.value, pair[1]] : [pair[0], e.target.value])}
            >
              <option value="">Choisir…</option>
              {chosen.map((m) => (
                <option key={m.userId} value={m.userId}>
                  {m.displayName}
                </option>
              ))}
            </select>
          ))}
          <button type="button" onClick={addExclusion} className="rounded-xl border px-3 py-2 text-sm font-bold">
            Ajouter
          </button>
        </div>
        <ul className="mt-2 space-y-1">
          {exclusions
            .filter(([a, b]) => selected.has(a) && selected.has(b))
            .map(([a, b]) => (
              <li key={`${a}/${b}`} className="flex items-center gap-2 text-sm">
                <span>
                  {names.get(a)} ✕ {names.get(b)}
                </span>
                <button
                  type="button"
                  className="text-xs font-bold text-rose-700 underline"
                  onClick={() => setExclusions(exclusions.filter(([x, y]) => x !== a || y !== b))}
                >
                  Retirer
                </button>
              </li>
            ))}
        </ul>
      </fieldset>

      {error && (
        <p role="alert" className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      )}

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="submit"
          disabled={busy || !staff || chosen.length < SANTA_MIN_PARTICIPANTS}
          className="rounded-xl bg-red-600 px-6 py-3 text-sm font-bold text-white disabled:opacity-50"
        >
          {busy ? "Enregistrement…" : submitLabel}
        </button>
        <button type="button" onClick={onCancel} className="text-sm font-bold text-slate-500 underline">
          Annuler
        </button>
        {staff && chosen.length < SANTA_MIN_PARTICIPANTS && (
          <span className="text-xs text-slate-500">Il faut au moins {SANTA_MIN_PARTICIPANTS} participants.</span>
        )}
      </div>
    </form>
  );
}
