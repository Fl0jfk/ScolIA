/** Ouvre ScolIA avec une question préremplie (événement navigateur). */

export const SCOLIA_ASK_EVENT = "scolia:ask";
export const SCOLIA_OPEN_EVENT = "scolia:open";

export type ScoliaAskDetail = {
  prompt: string;
  /** Si true (défaut), envoie immédiatement la question. */
  autoSend?: boolean;
};

export type ScoliaOpenDetail = {
  /** Texte prérempli sans envoi auto. */
  draft?: string;
};

export function askScolia(prompt: string, opts?: { autoSend?: boolean }) {
  if (typeof window === "undefined") return;
  const text = prompt.trim();
  if (!text) return;
  window.dispatchEvent(
    new CustomEvent<ScoliaAskDetail>(SCOLIA_ASK_EVENT, {
      detail: { prompt: text, autoSend: opts?.autoSend !== false },
    }),
  );
}

/** Ouvre la fenêtre ScolIA (optionnellement avec brouillon). */
export function openScolia(opts?: { draft?: string }) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent<ScoliaOpenDetail>(SCOLIA_OPEN_EVENT, {
      detail: { draft: opts?.draft?.trim() || undefined },
    }),
  );
}
