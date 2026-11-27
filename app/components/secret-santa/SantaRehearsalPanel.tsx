"use client";

import { useState } from "react";
import type { SantaRehearsal } from "@/app/lib/secret-santa-repetition";

export type RehearsalList = { id: string; title: string; participantIds: string[] };

const norm = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

export default function SantaRehearsalPanel({
  staff,
  lists,
  rulesVersion,
}: {
  staff: { userId: string; displayName: string }[];
  /** Les Secret Santa organisés par l'administrateur, pour reprendre leur liste. */
  lists: RehearsalList[];
  /** Change quand les règles changent : un ancien résultat ne doit pas rester affiché. */
  rulesVersion: string;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [run, setRun] = useState<{ result: SantaRehearsal; number: number; version: string } | null>(null);
  const [count, setCount] = useState(0);

  const names = new Map(staff.map((m) => [m.userId, m.displayName]));
  const nameOf = (id: string) => names.get(id) ?? "Personne inconnue";
  const visible = staff.filter((m) => norm(m.displayName).includes(norm(search.trim())));
  const shown = run && run.version === rulesVersion ? run : null;

  const change = (next: Set<string>) => {
    setSelected(next);
    setRun(null);
    setError(null);
  };
  const setMany = (ids: string[], on: boolean) => {
    const next = new Set(selected);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
    }
    change(next);
  };

  async function rehearse() {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/secret-santa/incompatibilites/repetition", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ participantIds: [...selected].filter((id) => names.has(id)) }),
      });
      const result = await res.json();
      if (!res.ok) throw new Error(result?.error || "Répétition impossible.");
      setRun({ result, number: count + 1, version: rulesVersion });
      setCount(count + 1);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Répétition impossible.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="rounded-2xl border border-dashed border-slate-400 bg-white p-4">
      <h3 className="font-bold text-slate-900">🧪 Répétition (faux tirage)</h3>
      <p className="mt-1 text-sm text-slate-600">
        Pour vérifier vos séparations. Rien n&apos;est enregistré ni envoyé, et cela n&apos;a aucun rapport avec le
        vrai tirage, qui sera refait au hasard.
      </p>

      <div className="mt-3 flex flex-wrap gap-2">
        {lists.length > 0 && (
          <select
            value=""
            aria-label="Reprendre la liste d'un de mes Secret Santa"
            onChange={(e) => {
              const list = lists.find((l) => l.id === e.target.value);
              if (list) change(new Set(list.participantIds.filter((id) => names.has(id))));
            }}
            className="min-w-0 max-w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
          >
            <option value="">Reprendre la liste d&apos;un de mes Secret Santa…</option>
            {lists.map((l) => (
              <option key={l.id} value={l.id}>
                {l.title} ({l.participantIds.length})
              </option>
            ))}
          </select>
        )}
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Rechercher un nom…"
          aria-label="Rechercher une personne pour la répétition"
          className="min-w-0 flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
        />
      </div>
      <div className="mt-2 flex flex-wrap gap-3 text-xs font-bold">
        <button type="button" className="text-red-700 underline" onClick={() => setMany(visible.map((m) => m.userId), true)}>
          Cocher les {visible.length} affiché(s)
        </button>
        <button type="button" className="text-slate-500 underline" onClick={() => change(new Set())}>
          Tout décocher
        </button>
      </div>
      <ul className="mt-2 grid max-h-56 gap-1 overflow-y-auto rounded-xl border border-slate-200 p-2 sm:grid-cols-2">
        {visible.map((m) => (
          <li key={m.userId}>
            <label className="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-sm hover:bg-slate-50">
              <input type="checkbox" checked={selected.has(m.userId)} onChange={(e) => setMany([m.userId], e.target.checked)} />
              {m.displayName}
            </label>
          </li>
        ))}
      </ul>

      <button
        type="button"
        disabled={busy || selected.size < 3}
        onClick={() => void rehearse()}
        className="mt-3 rounded-xl bg-slate-900 px-5 py-2 text-sm font-bold text-white disabled:opacity-50"
      >
        {busy ? "Faux tirage…" : `${count ? "Refaire" : "Lancer"} un faux tirage (${selected.size} personnes)`}
      </button>
      {error && (
        <p role="alert" className="mt-2 text-sm text-rose-700">
          {error}
        </p>
      )}
      {shown && <RehearsalResult result={shown.result} number={shown.number} nameOf={nameOf} />}
    </div>
  );
}

export function RehearsalResult({
  result,
  number,
  nameOf,
}: {
  result: SantaRehearsal;
  number: number;
  nameOf: (id: string) => string;
}) {
  if (!result.ok) {
    return (
      <div role="status" className="mt-3 rounded-xl border border-amber-300 bg-amber-50 p-3 text-sm text-amber-900">
        <p className="font-bold">
          {result.reason === "not_enough"
            ? "Il faut au moins 3 personnes."
            : "Avec ce groupe, le vrai tirage serait bloqué."}
        </p>
        {result.overLimit.length > 0 && (
          <>
            <p className="mt-1">
              Maximum à cette taille : {result.maxPerPerson} personne(s) à éviter. Au-dessus :
            </p>
            <ul className="mt-1 list-disc pl-5">
              {result.overLimit.map((p) => (
                <li key={p.userId}>
                  {nameOf(p.userId)} ({p.count})
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    );
  }
  const pairs = [...result.pairs].sort((a, b) => nameOf(a.giver).localeCompare(nameOf(b.giver), "fr"));
  const flagged = new Set(result.rules.flat());
  return (
    <div role="status" className="mt-3 space-y-3">
      <p
        className={`rounded-xl border p-3 text-sm font-bold ${
          result.violations ? "border-rose-300 bg-rose-50 text-rose-800" : "border-emerald-300 bg-emerald-50 text-emerald-800"
        }`}
      >
        {result.violations
          ? `⚠️ ${result.violations} faux tirage(s) ne respectent pas une séparation : prévenez le support.`
          : result.rules.length
            ? `✅ ${result.rules.length} séparation(s) concernent ce groupe : toutes respectées sur ce tirage et sur ${result.checked} autres vérifiés.`
            : "Aucune séparation ne concerne les personnes cochées."}
      </p>
      {result.rules.length > 0 && (
        <ul className="text-xs text-slate-600">
          {result.rules.map(([a, b]) => (
            <li key={`${a}/${b}`}>
              {nameOf(a)} ✕ {nameOf(b)}
            </li>
          ))}
        </ul>
      )}
      <div>
        <p className="text-xs font-bold uppercase text-slate-500">Faux tirage n° {number} (à qui chacun offrirait)</p>
        <ul className="mt-1 grid gap-1 text-sm sm:grid-cols-2">
          {pairs.map((p) => (
            <li key={p.giver} className={`rounded-lg px-3 py-1 ${flagged.has(p.giver) ? "bg-amber-50" : "bg-slate-50"}`}>
              {nameOf(p.giver)} → <strong>{nameOf(p.receiver)}</strong>
            </li>
          ))}
        </ul>
        {result.rules.length > 0 && (
          <p className="mt-1 text-xs text-slate-500">En jaune : les personnes concernées par une séparation.</p>
        )}
      </div>
    </div>
  );
}
