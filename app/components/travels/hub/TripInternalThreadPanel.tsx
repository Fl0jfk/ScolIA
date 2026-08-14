"use client";

import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripButton, TripSection, TripTextarea } from "@/app/components/travels/TripDetailUI";

export function TripInternalThreadPanel({
  trip, draftMessage, setDraftMessage, postInternalMessage,
}: {
  trip: TravelsTrip;
  draftMessage: string;
  setDraftMessage: (v: string) => void;
  postInternalMessage: () => void;
}) {
  return (
        <TripSection
          title="Fil interne"
          subtitle="Échanges entre créateur, direction et comptabilité"
          icon="💬"
          action={
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400">
              {(trip.messages || []).length} message{(trip.messages || []).length > 1 ? "s" : ""}
            </span>
          }
        >
          <div className="max-h-72 overflow-y-auto rounded-xl border border-slate-100 bg-slate-50/80 p-4 space-y-3 mb-4">
            {(trip.messages || []).length === 0 ? (
              <p className="text-sm text-slate-400 italic text-center py-6">Aucun message pour le moment.</p>
            ) : (
              [...(trip.messages || [])]
                .sort((a: any, b: any) => new Date(a.date).getTime() - new Date(b.date).getTime())
                .map((msg: any) => (
                  <div key={msg.id || `${msg.user}_${msg.date}`} className="bg-white border border-slate-100 rounded-xl p-4 shadow-sm">
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-xs font-bold text-slate-800">
                        {msg.user}{" "}
                        <span className="text-slate-400 font-medium">· {msg.role || "—"}</span>
                      </p>
                      <p className="text-[10px] text-slate-400">{new Date(msg.date).toLocaleString("fr-FR")}</p>
                    </div>
                    <p className="text-sm text-slate-700 mt-2 whitespace-pre-wrap leading-relaxed">{msg.text}</p>
                  </div>
                ))
            )}
          </div>
          <TripTextarea
            value={draftMessage}
            onChange={(e) => setDraftMessage(e.target.value)}
            placeholder="Message interne… (ex. : proposer une alternative d'hébergement)"
          />
          <div className="flex justify-end mt-3">
            <TripButton onClick={postInternalMessage} disabled={!draftMessage.trim()}>
              Envoyer
            </TripButton>
          </div>
        </TripSection>

  );
}
