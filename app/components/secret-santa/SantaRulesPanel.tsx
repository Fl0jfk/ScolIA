"use client";

import { useCallback, useEffect, useState } from "react";
import SantaRehearsalPanel, { type RehearsalList } from "@/app/components/secret-santa/SantaRehearsalPanel";

type Data = {
  staff: { userId: string; displayName: string }[];
  rules: [string, string][];
  lists: RehearsalList[];
};

const URL = "/api/secret-santa/incompatibilites";

/**
 * Incompatibilités confidentielles. N'apparaît que si le serveur reconnaît un administrateur ;
 * la liste reste fermée tant qu'on ne l'ouvre pas, à l'abri d'un regard par-dessus l'épaule.
 */
export default function SantaRulesPanel() {
  const [data, setData] = useState<Data | null>(null);
  const [open, setOpen] = useState(false);
  const [pair, setPair] = useState<[string, string]>(["", ""]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const res = await fetch(URL, { cache: "no-store" });
      if (res.ok) setData(await res.json());
    } catch {
      /* panneau simplement absent */
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (!data) return null;
  const names = new Map(data.staff.map((m) => [m.userId, m.displayName]));
  const nameOf = (id: string) => names.get(id) ?? "Ancien membre du personnel";

  async function change(method: "POST" | "DELETE", a: string, b: string) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(URL, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ a, b }),
      });
      const body = await res.json();
      if (!res.ok) throw new Error(body?.error || "Action impossible.");
      if (method === "POST") setPair(["", ""]);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Action impossible.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="mt-8 rounded-2xl border border-slate-300 bg-slate-50 p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="font-bold text-slate-900">🔒 Incompatibilités confidentielles</h2>
          <p className="text-sm text-slate-600">Visible des administrateurs seulement.</p>
        </div>
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen((v) => !v)}
          className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold"
        >
          {open ? "Masquer" : "Gérer"}
        </button>
      </div>

      {open && (
        <div className="mt-4 space-y-3">
          <p className="text-sm text-slate-600">
            Deux personnes inscrites ici ne tomberont jamais l&apos;une sur l&apos;autre, dans un sens comme dans
            l&apos;autre, dans les tirages pas encore faits et dans les noms réattribués après un désistement. Personne d&apos;autre ne voit cette liste, ni son
            nombre de lignes. Vous ne voyez pas pour autant les tirages.
          </p>
          <p className="text-xs text-slate-500">
            Limite par personne selon le nombre de participants (règles de l&apos;organisateur comprises) : 4 à 6 → 1
            personne à éviter ; 7 à 9 → 2 ; 10 à 12 → 3 ; 13 à 15 → 4 ; 16 et plus → 5. À 3 participants, aucune. Si la
            limite est dépassée, le tirage est bloqué.
          </p>
          <div className="flex flex-wrap items-center gap-2">
            {[0, 1].map((side) => (
              <select
                key={side}
                value={pair[side]}
                aria-label={side === 0 ? "Première personne" : "Personne à éviter"}
                onChange={(e) => setPair(side === 0 ? [e.target.value, pair[1]] : [pair[0], e.target.value])}
                className="min-w-0 flex-1 rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
              >
                <option value="">Choisir…</option>
                {data.staff.map((m) => (
                  <option key={m.userId} value={m.userId}>
                    {m.displayName}
                  </option>
                ))}
              </select>
            ))}
            <button
              type="button"
              disabled={busy || !pair[0] || !pair[1] || pair[0] === pair[1]}
              onClick={() => void change("POST", pair[0], pair[1])}
              className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              Séparer
            </button>
          </div>
          {error && (
            <p role="alert" className="text-sm text-rose-700">
              {error}
            </p>
          )}
          {data.rules.length === 0 ? (
            <p className="text-sm text-slate-500">Aucune incompatibilité.</p>
          ) : (
            <ul className="space-y-1">
              {data.rules.map(([a, b]) => (
                <li key={`${a}/${b}`} className="flex flex-wrap items-center gap-2 rounded-lg bg-white px-3 py-1.5 text-sm">
                  <span>
                    {nameOf(a)} ✕ {nameOf(b)}
                  </span>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void change("DELETE", a, b)}
                    className="text-xs font-bold text-rose-700 underline"
                  >
                    Retirer
                  </button>
                </li>
              ))}
            </ul>
          )}
          <SantaRehearsalPanel staff={data.staff} lists={data.lists} rulesVersion={JSON.stringify(data.rules)} />
        </div>
      )}
    </section>
  );
}
