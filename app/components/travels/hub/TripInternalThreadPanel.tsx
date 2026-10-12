"use client";

import { useEffect } from "react";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripButton, TripSection, TripTextarea } from "@/app/components/travels/TripDetailUI";

export function TripInternalThreadPanel({
  trip,
  draftMessage,
  setDraftMessage,
  postInternalMessage,
  sending,
  currentUserId,
  onMarkRead,
}: {
  trip: TravelsTrip;
  draftMessage: string;
  setDraftMessage: (v: string) => void;
  postInternalMessage: () => void;
  sending?: boolean;
  currentUserId?: string | null;
  onMarkRead?: () => void;
}) {
  const unread = trip.unreadInternalCount ?? 0;
  useEffect(() => {
    if (!onMarkRead) return;
    if (!(trip.unreadInternalCount && trip.unreadInternalCount > 0)) return;
    onMarkRead();
  }, [onMarkRead, trip.id, trip.unreadInternalCount]);

  const messages = [...(trip.messages || [])].sort(
    (a, b) => new Date(a.date).getTime() - new Date(b.date).getTime(),
  );

  return (
        <TripSection
          title="Fil interne"
          subtitle="Échanges entre créateur, direction et comptabilité — sans e-mail"
          icon="💬"
          action={
            <span className="flex items-center gap-2">
              {unread > 0 ? (
                <span className="rounded-full bg-[#FF3B30] px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-white">
                  {unread} non lu{unread > 1 ? "s" : ""}
                </span>
              ) : null}
              <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400">
                {messages.length} message{messages.length > 1 ? "s" : ""}
              </span>
            </span>
          }
        >
          <div className="max-h-72 overflow-y-auto rounded-xl border border-slate-100 bg-slate-50/80 p-4 space-y-3 mb-4">
            {messages.length === 0 ? (
              <p className="text-sm text-slate-400 italic text-center py-6">Aucun message pour le moment.</p>
            ) : (
              messages.map((msg) => {
                const mine = Boolean(
                  currentUserId &&
                    msg.authorUserId &&
                    msg.authorUserId === currentUserId,
                );
                return (
                  <div
                    key={msg.id || `${msg.user}_${msg.date}`}
                    className={`rounded-xl p-4 shadow-sm ${
                      mine
                        ? "border border-slate-200 bg-white"
                        : "border border-indigo-100 bg-white"
                    }`}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-xs font-bold text-slate-800">
                        {msg.user}{" "}
                        <span className="text-slate-400 font-medium">· {msg.role || "—"}</span>
                      </p>
                      <p className="text-[10px] text-slate-400">{new Date(msg.date).toLocaleString("fr-FR")}</p>
                    </div>
                    <p className="text-sm text-slate-700 mt-2 whitespace-pre-wrap leading-relaxed">{msg.text}</p>
                  </div>
                );
              })
            )}
          </div>
          <TripTextarea
            value={draftMessage}
            onChange={(e) => setDraftMessage(e.target.value)}
            placeholder="Message interne… (ex. : proposer une alternative d'hébergement)"
          />
          <div className="flex justify-end mt-3">
            <TripButton onClick={postInternalMessage} disabled={!draftMessage.trim() || sending}>
              {sending ? "Envoi…" : "Envoyer"}
            </TripButton>
          </div>
        </TripSection>

  );
}
