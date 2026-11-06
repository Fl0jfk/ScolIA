/**
 * Découpe une réponse ScolIA en texte et tableaux markdown (| a | b |), pour les afficher
 * en vrai tableau et les exporter en Excel. Pur, sans dépendance.
 */

export type MessageSegment =
  | { type: "text"; text: string }
  | { type: "table"; headers: string[]; rows: string[][] };

function splitRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|")) s = s.slice(0, -1);
  return s.split("|").map((c) => c.trim().replace(/\*\*(.+?)\*\*/g, "$1"));
}

const isRow = (l: string) => /^\s*\|.*\|\s*$/.test(l);
const isSeparator = (l: string) => /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/.test(l);

export function splitMessageSegments(content: string): MessageSegment[] {
  const lines = String(content || "").split("\n");
  const out: MessageSegment[] = [];
  let buffer: string[] = [];
  const flush = () => {
    if (buffer.length) out.push({ type: "text", text: buffer.join("\n") });
    buffer = [];
  };
  let i = 0;
  while (i < lines.length) {
    const line = lines[i]!;
    const next = lines[i + 1] ?? "";
    if (isRow(line) && isSeparator(next)) {
      const headers = splitRow(line);
      const rows: string[][] = [];
      i += 2;
      while (i < lines.length && isRow(lines[i]!)) {
        const cells = splitRow(lines[i]!);
        // Aligne sur le nombre de colonnes de l'en-tête.
        rows.push(headers.map((_, c) => cells[c] ?? ""));
        i += 1;
      }
      flush();
      out.push({ type: "table", headers, rows });
      continue;
    }
    buffer.push(line);
    i += 1;
  }
  flush();
  // Retire les blocs de texte vides créés autour des tableaux.
  return out.filter((s) => s.type === "table" || s.text.trim() !== "");
}

export function messageTables(content: string): Array<{ headers: string[]; rows: string[][] }> {
  return splitMessageSegments(content).filter(
    (s): s is Extract<MessageSegment, { type: "table" }> => s.type === "table",
  );
}
