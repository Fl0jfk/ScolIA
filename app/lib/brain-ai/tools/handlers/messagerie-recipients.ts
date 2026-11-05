/**
 * Résolution de destinataires par nom — pur (sans base), testable.
 * Les candidats viennent de `listMessagingUsers` (la liste que l'appli propose déjà).
 */

export type RecipientCandidate = {
  id: string;
  name: string;
  roles: string[];
};

const TITLE_TOKENS = new Set([
  "mme",
  "madame",
  "m",
  "mr",
  "monsieur",
  "mlle",
  "mademoiselle",
  "prof",
  "professeur",
  "professeure",
  "dr",
  "a",
  "au",
  "de",
  "du",
  "la",
  "le",
]);

export function normalizeName(raw: string): string {
  return raw
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9\s'-]/g, " ")
    .replace(/[-']/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function queryTokens(raw: string): string[] {
  return normalizeName(raw)
    .split(" ")
    .filter((t) => t && !TITLE_TOKENS.has(t));
}

export type RecipientMatch =
  | { kind: "one"; candidate: RecipientCandidate }
  | { kind: "many"; candidates: RecipientCandidate[] }
  | { kind: "none" };

/** Tous les mots de la requête doivent se retrouver dans le nom (ordre libre). */
export function matchRecipient(query: string, candidates: RecipientCandidate[]): RecipientMatch {
  const tokens = queryTokens(query);
  if (tokens.length === 0) return { kind: "none" };

  const scored = candidates
    .map((c) => ({ c, words: normalizeName(c.name).split(" ").filter(Boolean) }))
    .filter(({ words }) =>
      tokens.every((t) => words.some((w) => w === t || (t.length >= 3 && w.startsWith(t)))),
    );
  if (scored.length === 0) return { kind: "none" };
  if (scored.length === 1) return { kind: "one", candidate: scored[0].c };

  // Plusieurs : un nom exactement égal (mêmes mots) l'emporte s'il est unique.
  const exact = scored.filter(({ words }) => {
    const set = new Set(words);
    return tokens.length === words.length && tokens.every((t) => set.has(t));
  });
  if (exact.length === 1) return { kind: "one", candidate: exact[0].c };

  return { kind: "many", candidates: scored.map((s) => s.c).slice(0, 8) };
}

/** Texte final du message : l'objet (la messagerie n'en a pas) est placé en tête. */
export function composeMessageBody(subject: string, body: string): string {
  const s = subject.trim();
  const b = body.trim();
  return s ? `Objet : ${s}\n\n${b}` : b;
}
