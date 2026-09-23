"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

type FoyerLight = {
  id: string;
  label: string;
  eleves: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
};

type ThreadRow = {
  id: string;
  foyerId: string;
  eleveId: string | null;
  sujet: string;
  createdByNom: string | null;
  lastMessageAt: string;
  foyerLabel: string | null;
  eleveNom: string | null;
  elevePrenom: string | null;
};

type MessageRow = {
  id: string;
  auteurCote: string;
  auteurNom: string | null;
  corps: string;
  createdAt: string;
  luAt: string | null;
};

export default function MessagesFoyerStaffClient() {
  const [foyers, setFoyers] = useState<FoyerLight[]>([]);
  const [threads, setThreads] = useState<ThreadRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [foyerId, setFoyerId] = useState("");
  const [eleveId, setEleveId] = useState("");
  const [sujet, setSujet] = useState("");
  const [corps, setCorps] = useState("");
  const [reply, setReply] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const selectedFoyer = useMemo(
    () => foyers.find((f) => f.id === foyerId) || null,
    [foyers, foyerId],
  );

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/messages-foyer", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      const nextFoyers = (data.foyers || []) as FoyerLight[];
      setFoyers(nextFoyers);
      setThreads(data.threads || []);
      setFoyerId((prev) => prev || nextFoyers[0]?.id || "");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!selectedFoyer) {
      setEleveId("");
      return;
    }
    if (!selectedFoyer.eleves.some((e) => e.id === eleveId)) {
      setEleveId(selectedFoyer.eleves[0]?.id || "");
    }
  }, [selectedFoyer, eleveId]);

  const openThread = async (id: string) => {
    setSelectedId(id);
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/vie-scolaire/messages-foyer/${id}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setMessages(data.messages || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const createThread = async () => {
    setBusy(true);
    setError(null);
    setOk(null);
    try {
      const res = await fetch("/api/vie-scolaire/messages-foyer", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          foyerId,
          eleveId: eleveId || null,
          sujet,
          corps,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setSujet("");
      setCorps("");
      setOk("Message envoyé à la famille.");
      await load();
      if (data.thread?.id) await openThread(data.thread.id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const sendReply = async () => {
    if (!selectedId) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/messages-foyer", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "reply", threadId: selectedId, corps: reply }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setReply("");
      await openThread(selectedId);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title="Messages familles"
        description="Canal dédié établissement ↔ foyer (texte). Distinct du carnet et de la messagerie staff."
        actions={
          <Link href="/vie-scolaire/carnet" className="text-sm font-bold text-indigo-600 hover:underline">
            Carnet de liaison
          </Link>
        }
      />

      {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}
      {ok ? <p className="mb-3 text-sm font-semibold text-emerald-700">{ok}</p> : null}

      <div className="grid gap-4 lg:grid-cols-2">
        <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3 text-sm">
          <h2 className="font-bold text-slate-900">Nouveau message</h2>
          <label className={`block ${dash.textMid}`}>
            Foyer
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={foyerId}
              onChange={(e) => setFoyerId(e.target.value)}
            >
              {foyers.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.label}
                  {f.eleves.length
                    ? ` — ${f.eleves.map((e) => `${e.prenom} ${e.nom}`).join(", ")}`
                    : ""}
                </option>
              ))}
            </select>
          </label>
          {selectedFoyer?.eleves.length ? (
            <label className={`block ${dash.textMid}`}>
              Élève (contexte)
              <select
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={eleveId}
                onChange={(e) => setEleveId(e.target.value)}
              >
                {selectedFoyer.eleves.map((e) => (
                  <option key={e.id} value={e.id}>
                    {e.prenom} {e.nom}
                    {e.classe ? ` (${e.classe})` : ""}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          <label className={`block ${dash.textMid}`}>
            Sujet
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={sujet}
              onChange={(e) => setSujet(e.target.value)}
              placeholder="Ex. Rendez-vous CPE"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Message
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 min-h-[100px]"
              value={corps}
              onChange={(e) => setCorps(e.target.value)}
              placeholder="Texte destiné au foyer…"
            />
          </label>
          <button
            type="button"
            disabled={busy || !foyerId || !sujet.trim() || !corps.trim()}
            onClick={() => void createThread()}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
          >
            Envoyer au foyer
          </button>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-4">
          <h2 className="font-bold text-slate-900 mb-3">Conversations</h2>
          <ul className="space-y-2 text-sm max-h-[420px] overflow-auto">
            {threads.length === 0 ? (
              <li className={dash.textMid}>Aucune conversation pour l’instant.</li>
            ) : (
              threads.map((t) => (
                <li key={t.id}>
                  <button
                    type="button"
                    onClick={() => void openThread(t.id)}
                    className={`w-full text-left rounded-xl border px-3 py-2 ${
                      selectedId === t.id
                        ? "border-indigo-400 bg-indigo-50"
                        : "border-slate-200 bg-white hover:bg-slate-50"
                    }`}
                  >
                    <div className="font-bold text-slate-900">{t.sujet}</div>
                    <div className={`text-xs ${dash.textMid}`}>
                      {t.foyerLabel || "Foyer"}
                      {t.elevePrenom || t.eleveNom
                        ? ` · ${t.elevePrenom || ""} ${t.eleveNom || ""}`.trim()
                        : ""}
                    </div>
                  </button>
                </li>
              ))
            )}
          </ul>
        </section>
      </div>

      {selectedId ? (
        <section className="mt-4 rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
          <h2 className="font-bold text-slate-900">Fil de discussion</h2>
          <div className="max-h-72 overflow-auto space-y-2">
            {messages.map((m) => (
              <div
                key={m.id}
                className={`rounded-xl px-3 py-2 text-sm ${
                  m.auteurCote === "staff"
                    ? "bg-indigo-50 border border-indigo-100"
                    : "bg-emerald-50 border border-emerald-100"
                }`}
              >
                <div className="text-xs font-bold text-slate-600 mb-1">
                  {m.auteurCote === "staff" ? "Établissement" : "Famille"}
                  {m.auteurNom ? ` — ${m.auteurNom}` : ""}
                </div>
                <p className="text-slate-900 whitespace-pre-wrap">{m.corps}</p>
              </div>
            ))}
          </div>
          <div className="flex gap-2">
            <input
              className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={reply}
              onChange={(e) => setReply(e.target.value)}
              placeholder="Répondre…"
            />
            <button
              type="button"
              disabled={busy || !reply.trim()}
              onClick={() => void sendReply()}
              className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
            >
              Envoyer
            </button>
          </div>
        </section>
      ) : null}
    </ModulePageShell>
  );
}
