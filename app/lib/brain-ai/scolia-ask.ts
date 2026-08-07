/** Ouvre ScolIA avec une question préremplie (événement navigateur). */

export const SCOLIA_ASK_EVENT = "scolia:ask";

export type ScoliaAskDetail = {
  prompt: string;
  /** Si true (défaut), envoie immédiatement la question. */
  autoSend?: boolean;
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
