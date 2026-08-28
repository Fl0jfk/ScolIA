/** Utilitaires parsing XML Siècle (v5) — codes souvent en attributs sur la balise ouvrante. */

export type SiecleElement = {
  /** Attributs de la balise ouvrante, ex. `<MEF CODE_MEF="…">`. */
  attrs: string;
  /** Contenu interne (sans la balise ouvrante). */
  inner: string;
};

export function extractSiecleElements(xml: string, tag: string): SiecleElement[] {
  const re = new RegExp(`<${tag}\\b([^>]*)>([\\s\\S]*?)</${tag}>`, "gi");
  const out: SiecleElement[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(xml))) {
    out.push({ attrs: m[1] ?? "", inner: m[2] ?? "" });
  }
  return out;
}

export function attrValue(attrs: string, name: string): string {
  const re = new RegExp(`\\b${name}\\s*=\\s*"([^"]*)"`, "i");
  const m = attrs.match(re);
  return (m?.[1] ?? "").trim();
}

export function tagValue(block: string, tag: string): string {
  const m = block.match(new RegExp(`<${tag}[^>]*>([^<]*)</${tag}>`, "i"));
  return (m?.[1] ?? "").trim();
}

export function firstNonEmpty(...values: Array<string | undefined | null>): string {
  for (const v of values) {
    const t = String(v ?? "").trim();
    if (t) return t;
  }
  return "";
}

export function libelleFromBlock(
  block: string,
  fallback: string,
  tags = ["LIBELLE_EDITION", "LIBELLE_LONG", "LIBELLE_COURT", "LIBELLE"],
): string {
  return firstNonEmpty(...tags.map((t) => tagValue(block, t)), fallback);
}

export function codeFromElement(
  el: SiecleElement,
  attrNames: string[],
  innerTagNames: string[] = [],
): string {
  for (const a of attrNames) {
    const v = attrValue(el.attrs, a);
    if (v) return v;
  }
  for (const t of innerTagNames) {
    const v = tagValue(el.inner, t);
    if (v) return v;
  }
  return "";
}

export function parseSiecleDate(raw: string): string | undefined {
  const v = raw.trim();
  if (!v) return undefined;
  if (/^\d{4}-\d{2}-\d{2}$/.test(v)) return v;
  const fr = v.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  if (fr) return `${fr[3]}-${fr[2]}-${fr[1]}`;
  return undefined;
}

/** Décode buffer Siècle (souvent Latin-9 / ISO-8859-15). */
export function decodeSiecleBuffer(buf: ArrayBuffer | Buffer | Uint8Array): string {
  const bytes = buf instanceof Buffer ? buf : Buffer.from(buf as ArrayBuffer);
  const asUtf8 = bytes.toString("utf8");
  if (!asUtf8.includes("\uFFFD") && /<\?xml|BEE_|ELEVE/i.test(asUtf8.slice(0, 500))) {
    return asUtf8;
  }
  return bytes.toString("latin1");
}
