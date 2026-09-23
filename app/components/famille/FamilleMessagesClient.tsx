"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";
import { ACCEPT_FILE_INPUT } from "@/app/lib/messaging/constants";

type ThreadRow = {
  id: string;
  sujet: string;
  foyerLabel: string | null;
  eleveNom: string | null;
  elevePrenom: string | null;
  lastMessageAt: string;
  isBroadcast?: boolean;
};

type MessageRow = {
  id: string;
  auteurCote: string;
  auteurNom: string | null;
  corps: string;
  createdAt: string;
};

type AttachmentRow = {
  id: string;
  messageId: string;
  fileName: string;
};

type PendingAtt = {
  fileName: string;
  mime: string;
  size: number;
  s3Key?: string | null;
  contentBase64?: string | null;
};

type NotifRow = {
  id: string;
  titre: string;
  preview: string;
  threadId: string | null;
  kind: string;
  luAt: string | null;
  createdAt: string;
};

export default function FamilleMessagesClient() {
  const [threads, setThreads] = useState<ThreadRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [attachments, setAttachments] = useState<AttachmentRow[]>([]);
  const [notifs, setNotifs] = useState<NotifRow[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [allowParentAttachments, setAllowParentAttachments] = useState(true);
  const [reply, setReply] = useState("");
  const [replyAtts, setReplyAtts] = useState<PendingAtt[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const [res, nRes] = await Promise.all([
        fetch("/api/famille/messages", { cache: "no-store" }),
        fetch("/api/famille/notifications?unread=1", { cache: "no-store" }),
      ]);
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setThreads(data.threads || []);
      setUnreadCount(Number(data.unreadCount || 0));
      setAllowParentAttachments(data.allowParentAttachments !== false);
      if (nRes.ok) {
        const nj = await nRes.json();
        setNotifs(nj.notifs || []);
        if (typeof nj.unreadCount === "number") setUnreadCount(nj.unreadCount);
      }
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
      setAttachments(data.attachments || []);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const uploadFiles = async (files: FileList | null) => {
    if (!files?.length) return [] as PendingAtt[];
    const out: PendingAtt[] = [];
    for (const file of Array.from(files)) {
      const fd = new FormData();
      fd.set("file", file);
      if (selectedId) fd.set("threadId", selectedId);
      const res = await fetch("/api/famille/messages/upload", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || `Upload ${file.name} impossible`);
      out.push(data.attachment as PendingAtt);
    }
    return out;
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
        body: JSON.stringify({
          action: "reply",
          threadId: selectedId,
          corps: reply,
          attachments: replyAtts,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setReply("");
      setReplyAtts([]);
      setOk("Réponse envoyée.");
      await openThread(selectedId);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const attsFor = (messageId: string) => attachments.filter((a) => a.messageId === messageId);

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900">
      <div className="mx-auto max-w-3xl px-4 py-8">
        <header className="mb-6">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Portail familles</p>
          <h1 className="text-2xl font-black">
            Messages
            {unreadCount > 0 ? (
              <span className="ml-2 inline-flex items-center rounded-full bg-rose-600 px-2 py-0.5 text-xs font-bold text-white">
                {unreadCount} nouveau{unreadCount > 1 ? "x" : ""}
              </span>
            ) : null}
          </h1>
          <p className="text-sm text-slate-600 mt-1">
            Échanges avec l’établissement (texte + pièces). Notifications in-app ci-dessous.
          </p>
          <FamilleNav unreadMessages={unreadCount} />
        </header>

        {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}
        {ok ? <p className="mb-3 text-sm font-semibold text-emerald-700">{ok}</p> : null}

        {notifs.length > 0 ? (
          <section className="mb-4 rounded-2xl border border-rose-200 bg-rose-50 p-3 text-sm">
            <h2 className="font-bold text-rose-900 mb-2">Notifications</h2>
            <ul className="space-y-1">
              {notifs.slice(0, 5).map((n) => (
                <li key={n.id}>
                  <button
                    type="button"
                    className="text-left w-full hover:underline"
                    onClick={() => n.threadId && void openThread(n.threadId)}
                  >
                    <span className="font-bold">{n.titre}</span>
                    {n.preview ? <span className="text-slate-600"> — {n.preview}</span> : null}
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

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
                      <div className="font-bold">
                        {t.isBroadcast ? "📢 " : ""}
                        {t.sujet}
                      </div>
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
                      {attsFor(m.id).length ? (
                        <ul className="mt-2 space-y-1">
                          {attsFor(m.id).map((a) => (
                            <li key={a.id}>
                              <a
                                href={`/api/famille/messages/attachment/${a.id}`}
                                target="_blank"
                                rel="noreferrer"
                                className="text-xs font-bold text-indigo-700 underline"
                              >
                                📎 {a.fileName}
                              </a>
                            </li>
                          ))}
                        </ul>
                      ) : null}
                    </div>
                  ))}
                </div>
                {allowParentAttachments ? (
                  <input
                    type="file"
                    multiple
                    accept={ACCEPT_FILE_INPUT}
                    className="block w-full text-xs"
                    onChange={(e) => {
                      void (async () => {
                        try {
                          const uploaded = await uploadFiles(e.target.files);
                          setReplyAtts((prev) => [...prev, ...uploaded]);
                        } catch (err) {
                          setError(err instanceof Error ? err.message : "Upload impossible");
                        } finally {
                          e.target.value = "";
                        }
                      })();
                    }}
                  />
                ) : null}
                {replyAtts.length ? (
                  <ul className="text-xs text-slate-600 list-disc pl-4">
                    {replyAtts.map((a, i) => (
                      <li key={`${a.fileName}-${i}`}>{a.fileName}</li>
                    ))}
                  </ul>
                ) : null}
                <div className="flex gap-2">
                  <input
                    className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                    value={reply}
                    onChange={(e) => setReply(e.target.value)}
                    placeholder="Répondre à l’établissement…"
                  />
                  <button
                    type="button"
                    disabled={busy || (!reply.trim() && !replyAtts.length)}
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
