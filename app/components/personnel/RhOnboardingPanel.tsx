"use client";

import { useCallback, useEffect, useState } from "react";
import {
  RH_ONBOARDING_STATUS_LABELS,
  type RhOnboardingRecord,
} from "@/app/lib/rh/onboarding-types";

export default function RhOnboardingPanel() {
  const [records, setRecords] = useState<RhOnboardingRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [emailHint, setEmailHint] = useState("");
  const [lastLink, setLastLink] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/onboarding", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setRecords(j.records || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const createLink = async () => {
    setBusyId("create");
    setError(null);
    try {
      const res = await fetch("/api/rh/onboarding", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ candidateEmailHint: emailHint || null }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Création impossible");
      setLastLink(j.publicUrl as string);
      setEmailHint("");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  };

  const validate = async (id: string) => {
    if (!confirm("Valider ce dossier ? Création OneDrive + PDF + invitation.")) return;
    setBusyId(id);
    try {
      const res = await fetch(`/api/rh/onboarding/${id}/validate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Validation impossible");
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  };

  const activate = async (id: string) => {
    setBusyId(id);
    try {
      const res = await fetch(`/api/rh/onboarding/${id}/activate`, { method: "POST" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Activation impossible");
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  };

  const copyLink = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      alert("Lien copié.");
    } catch {
      prompt("Copiez le lien :", url);
    }
  };

  return (
    <div className="space-y-6">
      <section className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <h3 className="font-black text-slate-900">Nouvel arrivant — lien public</h3>
        <p className="text-sm text-slate-600">
          Générez un lien à envoyer au futur collaborateur. Il remplit le formulaire ; vous validez
          ensuite (OneDrive, documents PDF, invitation).
        </p>
        <div className="flex flex-wrap gap-2 items-end">
          <label className="flex-1 min-w-[200px] space-y-1">
            <span className="text-xs font-bold text-slate-500 uppercase">E-mail indicatif (optionnel)</span>
            <input
              value={emailHint}
              onChange={(e) => setEmailHint(e.target.value)}
              type="email"
              className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              placeholder="prenom.nom@…"
            />
          </label>
          <button
            type="button"
            disabled={busyId === "create"}
            onClick={() => void createLink()}
            className="rounded-xl bg-indigo-600 text-white font-bold px-4 py-2.5 text-sm hover:bg-indigo-700 disabled:opacity-50"
          >
            Générer un lien
          </button>
        </div>
        {lastLink && (
          <div className="rounded-xl bg-indigo-50 border border-indigo-100 p-3 text-sm flex flex-wrap gap-2 items-center justify-between">
            <code className="break-all text-indigo-950">{lastLink}</code>
            <button
              type="button"
              onClick={() => void copyLink(lastLink)}
              className="text-xs font-bold text-indigo-700 hover:underline"
            >
              Copier
            </button>
          </div>
        )}
      </section>

      {error && (
        <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">
          {error}
        </p>
      )}

      <section className="bg-white rounded-2xl border border-slate-200 p-5">
        <h3 className="font-black text-slate-900 mb-3">Parcours en cours</h3>
        {loading ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : records.length === 0 ? (
          <p className="text-sm text-slate-500">Aucun parcours pour l&apos;instant.</p>
        ) : (
          <ul className="space-y-3">
            {records.map((r) => {
              const name =
                r.form?.firstName && r.form?.lastName
                  ? `${r.form.firstName} ${r.form.lastName}`
                  : r.candidateEmailHint || r.id;
              const publicUrl =
                typeof window !== "undefined"
                  ? `${window.location.origin}/onboarding-rh/${r.token}`
                  : `/onboarding-rh/${r.token}`;
              return (
                <li
                  key={r.id}
                  className="rounded-xl border border-slate-100 p-4 flex flex-wrap gap-3 justify-between items-start"
                >
                  <div>
                    <p className="font-bold text-slate-900">{name}</p>
                    <p className="text-xs text-slate-500 mt-1">
                      {RH_ONBOARDING_STATUS_LABELS[r.status]}
                      {r.form?.email ? ` · ${r.form.email}` : ""}
                    </p>
                    {r.status === "awaiting_candidate" && (
                      <button
                        type="button"
                        onClick={() => void copyLink(publicUrl)}
                        className="text-xs text-indigo-600 font-bold mt-2 hover:underline"
                      >
                        Copier le lien candidat
                      </button>
                    )}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {r.status === "submitted" && (
                      <button
                        type="button"
                        disabled={busyId === r.id}
                        onClick={() => void validate(r.id)}
                        className="rounded-lg bg-slate-900 text-white text-xs font-bold px-3 py-2"
                      >
                        Valider RH
                      </button>
                    )}
                    {r.status === "provisioned" && (
                      <button
                        type="button"
                        disabled={busyId === r.id}
                        onClick={() => void activate(r.id)}
                        className="rounded-lg bg-emerald-600 text-white text-xs font-bold px-3 py-2"
                      >
                        Activer compte
                      </button>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}
