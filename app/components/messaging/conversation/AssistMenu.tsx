"use client";

import { useCallback, useEffect, useRef, useState } from "react";

type AssistMenuProps = {
  conversationId: string;
  draft: string;
  onDraft: (text: string) => void;
  onSummary: (text: string) => void;
  disabled?: boolean;
  className?: string;
};

type Tone = "neutre" | "chaleureux" | "formel" | "court";
type View = "main" | "reply" | "rewrite";

const REWRITES: Array<{ label: string; tone: Tone; instruction?: string }> = [
  { label: "Corriger", tone: "neutre", instruction: "Corrige uniquement les fautes, sans changer le style." },
  { label: "Plus poli", tone: "formel" },
  { label: "Plus court", tone: "court" },
  { label: "Plus chaleureux", tone: "chaleureux" },
];

function SparkIcon({ className = "h-4 w-4" }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3z" />
      <path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8L19 15z" />
    </svg>
  );
}

export default function AssistMenu({
  conversationId,
  draft,
  onDraft,
  onSummary,
  disabled,
  className = "",
}: AssistMenuProps) {
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<View>("main");
  const [instruction, setInstruction] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const abortRef = useRef<AbortController | null>(null);
  const hasDraft = draft.trim().length > 0;

  const close = useCallback(() => {
    setOpen(false);
    setView("main");
    setInstruction("");
    setError(null);
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        close();
        buttonRef.current?.focus();
      }
    };
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) close();
    };
    document.addEventListener("keydown", onKey, true);
    document.addEventListener("mousedown", onDown);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      document.removeEventListener("mousedown", onDown);
    };
  }, [open, close]);

  useEffect(() => () => abortRef.current?.abort(), []);

  async function run(body: Record<string, unknown>, kind: "draft" | "summary") {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/messaging/conversations/${encodeURIComponent(conversationId)}/assist`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
          signal: ctrl.signal,
        },
      );
      const data = (await res.json().catch(() => null)) as { text?: string; error?: string } | null;
      if (!res.ok || !data?.text) {
        setError(data?.error || "Impossible de rédiger pour le moment.");
        return;
      }
      if (kind === "summary") onSummary(data.text);
      else onDraft(data.text);
      close();
    } catch (e) {
      if ((e as Error)?.name !== "AbortError") setError("Connexion impossible. Réessayez.");
    } finally {
      if (abortRef.current === ctrl) setLoading(false);
    }
  }

  const itemClass =
    "flex w-full items-center rounded-xl px-3 py-2 text-left text-sm text-[var(--dash-ink)] transition hover:bg-[#eceeea] focus:bg-[#eceeea] focus:outline-none disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent";

  return (
    <div ref={rootRef} className={`relative ${className}`}>
      <button
        ref={buttonRef}
        type="button"
        disabled={disabled}
        aria-label="Rédiger avec ScolIA"
        title="Rédiger avec ScolIA"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => (open ? close() : setOpen(true))}
        className="flex h-9 w-9 items-center justify-center rounded-2xl text-[var(--dash-mid)] transition hover:bg-[#eceeea] hover:text-[var(--dash-ink)] disabled:cursor-not-allowed disabled:opacity-40"
      >
        <SparkIcon className={`h-4 w-4 ${loading ? "animate-pulse" : ""}`} />
      </button>

      {open ? (
        <div
          role="menu"
          aria-label="Rédiger avec ScolIA"
          className="absolute bottom-full left-0 z-50 mb-2 w-64 rounded-2xl border border-black/10 bg-white p-1.5 shadow-lg"
        >
          {loading ? (
            <p role="status" className="flex items-center gap-2 px-3 py-2 text-sm text-[var(--dash-mid)]">
              <SparkIcon className="h-4 w-4 animate-pulse text-[var(--dash-ink)]" />
              ScolIA rédige…
            </p>
          ) : view === "main" ? (
            <>
              <button type="button" role="menuitem" autoFocus className={itemClass} onClick={() => setView("reply")}>
                Proposer une réponse…
              </button>
              <button
                type="button"
                role="menuitem"
                disabled={!hasDraft}
                title={hasDraft ? undefined : "Écrivez d'abord un brouillon"}
                className={itemClass}
                onClick={() => setView("rewrite")}
              >
                Améliorer mon brouillon
              </button>
              <button
                type="button"
                role="menuitem"
                className={itemClass}
                onClick={() => void run({ mode: "summarize" }, "summary")}
              >
                Résumer la conversation
              </button>
            </>
          ) : view === "reply" ? (
            <form
              className="p-1.5"
              onSubmit={(e) => {
                e.preventDefault();
                void run(
                  { mode: "reply", ...(instruction.trim() ? { instruction: instruction.trim() } : {}), ...(hasDraft ? { draft } : {}) },
                  "draft",
                );
              }}
            >
              <label className="mb-1 block text-xs font-semibold text-[var(--dash-mid)]" htmlFor="assist-instruction">
                Que voulez-vous répondre ?
              </label>
              <input
                id="assist-instruction"
                autoFocus
                value={instruction}
                maxLength={500}
                onChange={(e) => setInstruction(e.target.value)}
                placeholder="facultatif, ex. : d'accord pour jeudi"
                className="w-full rounded-xl border border-black/10 bg-white px-3 py-2 text-sm text-[var(--dash-ink)] outline-none focus:border-[var(--dash-ink)]"
              />
              <div className="mt-2 flex justify-between gap-2">
                <button
                  type="button"
                  onClick={() => setView("main")}
                  className="rounded-xl px-3 py-1.5 text-xs text-[var(--dash-mid)] hover:bg-[#eceeea]"
                >
                  Retour
                </button>
                <button
                  type="submit"
                  className="rounded-xl bg-[var(--dash-ink)] px-3 py-1.5 text-xs font-semibold text-white hover:opacity-90"
                >
                  Rédiger
                </button>
              </div>
            </form>
          ) : (
            <div>
              <button
                type="button"
                onClick={() => setView("main")}
                className="px-3 py-1 text-xs text-[var(--dash-mid)] hover:underline"
              >
                ← Retour
              </button>
              {REWRITES.map((r, i) => (
                <button
                  key={r.label}
                  type="button"
                  role="menuitem"
                  autoFocus={i === 0}
                  className={itemClass}
                  onClick={() =>
                    void run(
                      { mode: "rewrite", draft, tone: r.tone, ...(r.instruction ? { instruction: r.instruction } : {}) },
                      "draft",
                    )
                  }
                >
                  {r.label}
                </button>
              ))}
            </div>
          )}

          {error ? (
            <p role="alert" className="px-3 pb-1 pt-2 text-xs text-red-600">
              {error}
            </p>
          ) : null}
          <p className="px-3 pb-1 pt-1.5 text-[11px] text-[var(--dash-mid)]">Relisez avant d&apos;envoyer.</p>
        </div>
      ) : null}
    </div>
  );
}
