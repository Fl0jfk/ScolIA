/** Normalisation caractère par caractère (sans accents, minuscules) : garde l'alignement des index. */
function foldChar(c: string): string {
  return c.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

export function foldText(s: string): string {
  let out = "";
  for (const c of s) out += foldChar(c);
  return out;
}

/** Plages [début, fin[ (index du texte d'origine) où `query` apparaît, sans accents ni casse. */
export function findMatches(text: string, query: string): Array<[number, number]> {
  const q = foldText(query.trim());
  if (!q || !text) return [];
  let folded = "";
  const map: number[] = []; // index replié -> index d'origine
  let i = 0;
  for (const c of text) {
    const f = foldChar(c);
    for (let k = 0; k < f.length; k++) map.push(i);
    folded += f;
    i += c.length;
  }
  const out: Array<[number, number]> = [];
  let from = 0;
  for (;;) {
    const idx = folded.indexOf(q, from);
    if (idx < 0) break;
    const start = map[idx];
    const endFolded = idx + q.length;
    const end = endFolded < map.length ? map[endFolded] : text.length;
    out.push([start, Math.max(end, start + 1)]);
    from = idx + Math.max(q.length, 1);
  }
  return out;
}

export function messageMatches(body: string | null | undefined, query: string): boolean {
  if (!body) return false;
  const q = foldText(query.trim());
  if (!q) return false;
  return foldText(body).includes(q);
}

const URL_RE = /https?:\/\/[^\s<>"']+/gi;

export type TextPiece =
  | { kind: "text"; text: string }
  | { kind: "link"; text: string; href: string };

/** Découpe un texte en morceaux texte / lien (http et https uniquement). */
export function splitLinks(text: string): TextPiece[] {
  const out: TextPiece[] = [];
  let last = 0;
  for (const m of text.matchAll(URL_RE)) {
    const idx = m.index ?? 0;
    let raw = m[0];
    // Ponctuation finale et parenthèse fermante non appariée : hors du lien.
    for (;;) {
      const tail = raw[raw.length - 1];
      if (!tail) break;
      if (/[.,;:!?'”»]/.test(tail)) raw = raw.slice(0, -1);
      else if (tail === ")" && !raw.includes("(")) raw = raw.slice(0, -1);
      else break;
    }
    let href: string | null = null;
    try {
      const u = new URL(raw);
      if (u.protocol === "http:" || u.protocol === "https:") href = u.href;
    } catch {
      href = null;
    }
    if (!href) continue;
    if (idx > last) out.push({ kind: "text", text: text.slice(last, idx) });
    out.push({ kind: "link", text: raw, href });
    last = idx + raw.length;
  }
  if (last < text.length) out.push({ kind: "text", text: text.slice(last) });
  return out;
}
