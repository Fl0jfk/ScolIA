"use client";

import { useEffect, useRef, useState } from "react";
import type { StageDiscussionMessage } from "@/app/lib/stage-types";

type Props = {
  /** Messages déjà connus (admin) ou chargés via token. */
  initialMessages?: StageDiscussionMessage[];
  /** Jeton du lien public — obligatoire hors intranet. */
  token?: string;
  /** Convention id — mode intranet authentifié. */
  conventionId?: string;
  /** Qui écrit (libellé affiché côté serveur aussi). */
  authorHint?: string;
  disabled?: boolean;
  title?: string;
};

export default function StageDiscussionChat({
  initialMessages = [],
  token,
  conventionId,
  authorHint,
  disabled = false,
  title = "Échanges sur la convention",
}: Props) {
  const [messages, setMessages] = useState<StageDiscussionMessage[]>(initialMessages);
  const [body, setBody] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(initialMessages.length > 0);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setMessages(initialMessages);
  }, [initialMessages]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length]);

  useEffect(() => {
    if (!token && !conventionId) return;
    let cancelled = false;
    async function load() {
      try {
        const url = token
          ? `/api/stages/public/sign?token=${encodeURIComponent(token)}&discussion=1`
          : `/api/stages/conventions/${conventionId}?discussion=1`;
        const res = await fetch(url, { cache: "no-store" });
        const data = await res.json();
        if (!res.ok) throw new Error(data?.error || "Impossible de charger la discussion.");
        if (!cancelled) {
          setMessages(Array.isArray(data.discussionMessages) ? data.discussionMessages : []);
          setLoaded(true);
        }
      } catch (e: unknown) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "Erreur de chargement");
          setLoaded(true);
        }
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [token, conventionId]);

  async function send() {
    const text = body.trim();
    if (!text || busy || disabled) return;
    setBusy(true);
    setError(null);
    try {
      let res: Response;
      if (token) {
        res = await fetch("/api/stages/public/sign", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "post_discussion_message",
            token,
            body: text,
          }),
        });
      } else if (conventionId) {
        res = await fetch(`/api/stages/conventions/${conventionId}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "post_discussion_message",
            body: text,
            authorLabel: authorHint,
          }),
        });
      } else {
        throw new Error("Lien ou convention manquant.");
      }
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Envoi impossible.");
      const next = Array.isArray(data.discussionMessages)
        ? data.discussionMessages
        : data.message
          ? [...messages, data.message as StageDiscussionMessage]
          : messages;
      setMessages(next);
      setBody("");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="rounded-xl border border-stone-300 bg-white p-4 space-y-3">
      <div>
        <h3 className="text-sm font-black text-[#1F3D2B]">{title}</h3>
        <p className="mt-1 text-xs text-stone-600 leading-relaxed">
          Direction, responsable légal, tuteur en entreprise et professeur référent peuvent
          échanger ici — uniquement via ce lien sécurisé.
        </p>
      </div>

      <div className="max-h-64 overflow-y-auto rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 space-y-2">
        {!loaded ? (
          <p className="text-xs text-stone-500">Chargement…</p>
        ) : messages.length === 0 ? (
          <p className="text-xs text-stone-500">Aucun message pour le moment.</p>
        ) : (
          messages.map((m) => (
            <div key={m.id} className="rounded-lg bg-white border border-stone-200 px-3 py-2">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <span className="text-xs font-bold text-[#1F3D2B]">{m.authorLabel}</span>
                <span className="text-[10px] text-stone-500">
                  {new Date(m.at).toLocaleString("fr-FR", {
                    day: "2-digit",
                    month: "short",
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </span>
              </div>
              <p className="mt-1 text-sm text-stone-800 whitespace-pre-wrap">{m.body}</p>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>

      {error ? <p className="text-xs text-rose-700">{error}</p> : null}

      {!disabled ? (
        <div className="space-y-2">
          <textarea
            className="w-full rounded-lg border border-stone-300 px-3 py-2 text-sm min-h-[72px]"
            value={body}
            onChange={(e) => setBody(e.target.value)}
            placeholder="Écrire un message (avenant, horaires, dates…)"
            maxLength={2000}
            disabled={busy}
          />
          <button
            type="button"
            disabled={busy || !body.trim()}
            onClick={() => void send()}
            className="rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
          >
            {busy ? "Envoi…" : "Envoyer"}
          </button>
        </div>
      ) : null}
    </section>
  );
}
