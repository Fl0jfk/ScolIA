"use client";

import { useState, type FormEvent } from "react";
import { askScolia, openScolia } from "@/app/lib/brain-ai/scolia-ask";
import ScoliaAiMark from "@/app/components/ScoliaAiMark";

type Props = {
  onCloseMobile?: () => void;
};

export default function SidebarScoliaBlock({ onCloseMobile }: Props) {
  const [draft, setDraft] = useState("");

  function submit(e: FormEvent) {
    e.preventDefault();
    const text = draft.trim();
    onCloseMobile?.();
    if (text) {
      askScolia(text, { autoSend: true });
      setDraft("");
      return;
    }
    openScolia();
  }

  return (
    <div className="rounded-2xl bg-[var(--dash-lime)] p-2.5 shadow-sm">
      <div className="mb-2 flex items-center gap-2 px-0.5">
        <div className="flex h-7 w-7 items-center justify-center overflow-hidden rounded-full bg-[var(--dash-ink)]">
          <ScoliaAiMark size="sm" inverted fill />
        </div>
        <div className="min-w-0">
          <p className="text-xs font-bold text-[var(--dash-ink)]">ScolIA</p>
          <p className="text-[10px] font-medium text-[var(--dash-ink)]/70">Assistant</p>
        </div>
      </div>
      <form onSubmit={submit} className="flex gap-1.5">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Pose une question…"
          className="min-w-0 flex-1 rounded-xl border border-black/10 bg-white/90 px-2.5 py-1.5 text-xs font-medium text-[var(--dash-ink)] outline-none placeholder:text-neutral-400 focus:ring-2 focus:ring-black/10"
        />
        <button
          type="submit"
          className="shrink-0 rounded-xl bg-[var(--dash-ink)] px-2.5 py-1.5 text-[11px] font-bold text-white hover:brightness-110"
        >
          {draft.trim() ? "→" : "Ouvrir"}
        </button>
      </form>
    </div>
  );
}
