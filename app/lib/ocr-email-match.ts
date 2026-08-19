/** Emails normalisés (minuscules, sans espaces). */
export function normalizeOcrEmail(raw?: string | null): string {
  return String(raw ?? "").trim().toLowerCase();
}

export function collectOcrEmails(...values: Array<string | null | undefined>): string[] {
  const out = new Set<string>();
  for (const v of values) {
    const n = normalizeOcrEmail(v);
    if (n && n.includes("@")) out.add(n);
  }
  return [...out];
}

export function textContainsEmail(text: string, email: string): boolean {
  const e = normalizeOcrEmail(email);
  if (!e || !text) return false;
  return text.toLowerCase().includes(e);
}

export function matchEntriesByEmailInText<T>(
  text: string,
  entries: T[],
  getEmails: (entry: T) => string[],
): T[] {
  const hits: T[] = [];
  for (const entry of entries) {
    const emails = getEmails(entry);
    if (emails.some((e) => textContainsEmail(text, e))) hits.push(entry);
  }
  return hits;
}

export function matchExtractedEmailAgainstList<T>(
  extractedEmail: string | undefined,
  entries: T[],
  getEmails: (entry: T) => string[],
): T | null {
  const target = normalizeOcrEmail(extractedEmail);
  if (!target) return null;
  const hits = entries.filter((e) => getEmails(e).includes(target));
  return hits.length === 1 ? hits[0]! : null;
}
