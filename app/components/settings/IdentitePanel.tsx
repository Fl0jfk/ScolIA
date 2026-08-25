"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";

type Orphan = {
  id: string;
  nom: string;
  prenom: string;
  email: string;
  matchUserId: string | null;
  nameScore: number | null;
  nameAlert: boolean;
};

type Multi = {
  userId: string;
  email: string;
  name: string;
  memberships: Array<{ etablissementId: string; label: string; context: string }>;
};

type Summary = {
  orphanResponsables: number;
  matchableByEmail: number;
  nameAlerts: number;
  multiMembershipUsers: number;
};

export default function IdentitePanel() {
  const [summary, setSummary] = useState<Summary | null>(null);
  const [orphans, setOrphans] = useState<Orphan[]>([]);
  const [multi, setMulti] = useState<Multi[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/identite", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setSummary(data.summary || null);
      setOrphans(data.orphans || []);
      setMulti(data.multiMembership || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/identite", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      if (body.action === "backfill") {
        setMessage(
          `Rattachement : ${data.linked} liés, ${data.nameAlerts} alerte(s) nom, ${data.skipped} ignorés.`,
        );
      } else {
        setMessage(
          data.nameAlert
            ? "Compte rattaché — vérifier le nom (écart possible)."
            : "Compte rattaché et membership parent créé.",
        );
      }
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="font-black text-slate-900">Identité plateforme</h2>
        <p className="text-sm text-slate-600">
          Une personne = un compte ScolIA. L’e-mail est la clé de rapprochement ; le nom/prénom confirment
          le match. Les données métier restent par établissement via{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">user_membership</code>.
        </p>
        {summary ? (
          <div className="flex flex-wrap gap-2 text-xs font-bold">
            <span className="rounded-lg bg-amber-50 border border-amber-200 px-2.5 py-1 text-amber-900">
              {summary.orphanResponsables} responsable(s) sans compte
            </span>
            <span className="rounded-lg bg-indigo-50 border border-indigo-200 px-2.5 py-1 text-indigo-900">
              {summary.matchableByEmail} rattachable(s) par e-mail
            </span>
            {summary.nameAlerts > 0 ? (
              <span className="rounded-lg bg-rose-50 border border-rose-200 px-2.5 py-1 text-rose-900">
                {summary.nameAlerts} écart(s) de nom
              </span>
            ) : null}
            <span className="rounded-lg bg-emerald-50 border border-emerald-200 px-2.5 py-1 text-emerald-900">
              {summary.multiMembershipUsers} compte(s) multi-établissements
            </span>
          </div>
        ) : (
          <p className="text-sm text-slate-500">Chargement…</p>
        )}
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled={busy || !summary?.matchableByEmail}
            onClick={() => void post({ action: "backfill" })}
            className="rounded-xl bg-indigo-600 text-white px-4 py-2 text-sm font-bold disabled:opacity-50"
          >
            Rattacher tous les matchs e-mail
          </button>
          <Link
            href="/parametres?tab=utilisateurs"
            className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700 hover:bg-slate-50"
          >
            Utilisateurs
          </Link>
          <Link
            href="/parametres?tab=referentiel"
            className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700 hover:bg-slate-50"
          >
            Liste élèves / régimes
          </Link>
        </div>
      </section>

      {message && <p className="text-sm font-semibold text-emerald-700">{message}</p>}
      {error && <p className="text-sm text-red-600">{error}</p>}

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="font-black text-slate-900">Responsables orphelins</h2>
        <p className="text-sm text-slate-600">
          Fiches foyer avec e-mail mais sans <code className="text-xs">userId</code>. Un match e-mail
          crée le lien + membership parent sur cet établissement.
        </p>
        {orphans.length === 0 ? (
          <p className="text-sm text-emerald-700 font-semibold">Aucun orphelin — identité propre.</p>
        ) : (
          <ul className="divide-y divide-slate-100 border border-slate-100 rounded-xl overflow-hidden">
            {orphans.slice(0, 80).map((o) => (
              <li
                key={o.id}
                className="flex flex-wrap items-center justify-between gap-2 px-4 py-3 text-sm bg-white"
              >
                <div>
                  <p className="font-bold text-slate-900">
                    {o.prenom} {o.nom}
                  </p>
                  <p className="text-xs text-slate-500 font-mono">{o.email}</p>
                  {o.matchUserId ? (
                    <p
                      className={`text-xs mt-0.5 ${o.nameAlert ? "text-rose-700" : "text-emerald-700"}`}
                    >
                      {o.nameAlert
                        ? `Compte trouvé — écart de nom (score ${o.nameScore}/4)`
                        : `Compte trouvé — nom OK (score ${o.nameScore}/4)`}
                    </p>
                  ) : (
                    <p className="text-xs text-amber-700 mt-0.5">Pas encore de compte ScolIA</p>
                  )}
                </div>
                {o.matchUserId ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void post({ action: "attachOne", responsableId: o.id })}
                    className="rounded-lg bg-slate-900 text-white px-3 py-1.5 text-xs font-bold disabled:opacity-50"
                  >
                    Rattacher
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="font-black text-slate-900">Comptes multi-établissements</h2>
        <p className="text-sm text-slate-600">
          Preuve de fusion : un login, plusieurs contextes (staff / parent / élève) sans UX type
          ÉcoleDirecte multi-apps.
        </p>
        {multi.length === 0 ? (
          <p className="text-sm text-slate-500">Aucun compte multi-tenant pour l’instant.</p>
        ) : (
          <ul className="space-y-2">
            {multi.map((m) => (
              <li key={m.userId} className="rounded-xl border border-slate-100 px-4 py-3 text-sm">
                <p className="font-bold text-slate-900">{m.name}</p>
                <p className="text-xs font-mono text-slate-500">{m.email}</p>
                <p className="text-xs text-slate-600 mt-1">
                  {m.memberships.map((x) => `${x.label} (${x.context})`).join(" · ")}
                </p>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
