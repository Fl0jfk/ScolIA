"use client";

import type { AbsenceThreadMessage } from "@/app/lib/absences-types";

export function AbsenceInternalThreadPanel({
  messages,
  draft,
  onDraftChange,
  onSend,
  sending,
  disabled,
}: {
  messages: AbsenceThreadMessage[];
  draft: string;
  onDraftChange: (value: string) => void;
  onSend: () => void;
  sending?: boolean;
  disabled?: boolean;
}) {
  const sorted = [...(messages || [])].sort(
    (a, b) => new Date(a.at).getTime() - new Date(b.at).getTime(),
  );

  return (
    <div className="mt-3 rounded-xl border border-slate-200 bg-slate-50/80 p-3 space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div>
          <p className="text-[11px] font-black uppercase tracking-wider text-slate-600">
            Échanges sur cette absence
          </p>
          <p className="text-xs text-slate-500">
            Fil interne entre le déclarant, la direction et le traitement admin.
          </p>
        </div>
        <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400">
          {sorted.length} message{sorted.length > 1 ? "s" : ""}
        </span>
      </div>

      <div className="max-h-56 overflow-y-auto space-y-2">
        {sorted.length === 0 ? (
          <p className="text-sm text-slate-400 italic text-center py-4">
            Aucun message pour le moment. Posez une question ou précisez un point ici.
          </p>
        ) : (
          sorted.map((msg) => (
            <div
              key={msg.id}
              className="rounded-xl border border-slate-100 bg-white px-3 py-2.5 shadow-sm"
            >
              <div className="flex items-center justify-between gap-2">
                <p className="text-xs font-bold text-slate-800">
                  {msg.userName}{" "}
                  <span className="text-slate-400 font-medium">· {msg.roleLabel || "—"}</span>
                </p>
                <p className="text-[10px] text-slate-400 shrink-0">
                  {new Date(msg.at).toLocaleString("fr-FR")}
                </p>
              </div>
              <p className="text-sm text-slate-700 mt-1.5 whitespace-pre-wrap leading-relaxed">
                {msg.text}
              </p>
            </div>
          ))
        )}
      </div>

      {!disabled ? (
        <div className="space-y-2">
          <textarea
            value={draft}
            onChange={(e) => onDraftChange(e.target.value)}
            rows={2}
            placeholder="Écrire un message… (ex. : préciser un créneau, demander une précision)"
            className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-800 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-200"
          />
          <div className="flex justify-end">
            <button
              type="button"
              onClick={onSend}
              disabled={sending || !draft.trim()}
              className="px-4 py-2 rounded-xl bg-slate-800 hover:bg-slate-900 text-white font-bold text-sm disabled:opacity-50"
            >
              {sending ? "Envoi…" : "Envoyer"}
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
