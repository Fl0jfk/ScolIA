"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type ThreadRow = {
  id: string;
  sujet: string;
  foyerLabel: string | null;
  eleveNom: string | null;
  elevePrenom: string | null;
  lastMessageAt: string;
  createdByNom: string | null;
};

type MessageRow = {
  id: string;
  auteurCote: string;
  auteurNom: string | null;
  corps: string;
  createdAt: string;
};

export default function FamilleMessagesClient() {
  const [threads, setThreads] = useState<ThreadRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [reply, setReply] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/famille/messages", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setThreads(data.threads || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const openThread = async (id: string) => {
    setSelectedId(id);
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/famille/messages/${id}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setMessages(data.messages || []);
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
    setOk(null);
    try {
      const res = await fetch("/api/famille/messages", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "reply", threadId: selectedId, corps: reply }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setReply("");
      setOk("Réponse envoyée.");
      await openThread(selectedId);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900">
      <div className="mx-auto max-w-3xl px-4 py-8">
        <header className="mb-6">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Portail familles</p>
          <h1 className="text-2xl font-black">Messages</h1>
          <p className="text-sm text-slate-600 mt-1">
            Échanges avec l’établissement (texte). Le carnet reste le canal de signature.
          </p>
          <FamilleNav />
        </header>

        {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}
        {ok ? <p className="mb-3 text-sm font-semibold text-emerald-700">{ok}</p> : null}
        {busy && !threads.length ? <p className="text-sm text-slate-500">Chargement…</p> : null}

        <div className="grid gap-4 md:grid-cols-2">
          <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-bold mb-3">Conversations</h2>
            <ul className="space-y-2 text-sm">
              {threads.length === 0 ? (
                <li className="text-slate-500">Aucun message pour le moment.</li>
              ) : (
                threads.map((t) => (
                  <li key={t.id}>
                    <button
                      type="button"
                      onClick={() => void openThread(t.id)}
                      className={`w-full text-left rounded-xl border px-3 py-2 ${
                        selectedId === t.id
                          ? "border-indigo-400 bg-indigo-50"
                          : "border-slate-200 hover:bg-slate-50"
                      }`}
                    >
                      <div className="font-bold">{t.sujet}</div>
                      <div className="text-xs text-slate-500">
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

          <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-bold mb-3">Discussion</h2>
            {!selectedId ? (
              <p className="text-sm text-slate-500">Choisissez une conversation.</p>
            ) : (
              <div className="space-y-3">
                <div className="max-h-80 overflow-auto space-y-2">
                  {messages.map((m) => (
                    <div
                      key={m.id}
                      className={`rounded-xl px-3 py-2 text-sm ${
                        m.auteurCote === "parent"
                          ? "bg-emerald-50 border border-emerald-100"
                          : "bg-indigo-50 border border-indigo-100"
                      }`}
                    >
                      <div className="text-xs font-bold text-slate-600 mb-1">
                        {m.auteurCote === "parent" ? "Vous" : "Établissement"}
                        {m.auteurNom ? ` — ${m.auteurNom}` : ""}
                      </div>
                      <p className="whitespace-pre-wrap">{m.corps}</p>
                    </div>
                  ))}
                </div>
                <div className="flex gap-2">
                  <input
                    className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                    value={reply}
                    onChange={(e) => setReply(e.target.value)}
                    placeholder="Répondre à l’établissement…"
                  />
                  <button
                    type="button"
                    disabled={busy || !reply.trim()}
                    onClick={() => void sendReply()}
                    className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
                  >
                    Envoyer
                  </button>
                </div>
              </div>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
