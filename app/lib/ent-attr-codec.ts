/**
 * Codec EAV textuel : aplatit un objet JS en chemins → valeurs texte,
 * et reconstruit sans jamais stocker de jsonb / fichier JSON.
 */
export type AttrPair = { path: string; value: string };

const MAX_DEPTH = 12;

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return !!v && typeof v === "object" && !Array.isArray(v) && !(v instanceof Date);
}

/** Sérialise une valeur primitive ou un nœud terminal. */
export function encodeLeaf(value: unknown): string {
  if (value === null) return "null:null";
  if (value === undefined) return "undef:";
  if (typeof value === "string") return `s:${value}`;
  if (typeof value === "number") return `n:${String(value)}`;
  if (typeof value === "boolean") return `b:${value ? "1" : "0"}`;
  // Objets/tableaux trop profonds : fallback texte JSON (pas une colonne jsonb).
  return `j:${JSON.stringify(value)}`;
}

function decodeLeaf(raw: string): unknown {
  if (raw === "null:null") return null;
  if (raw === "undef:") return undefined;
  if (raw.startsWith("s:")) return raw.slice(2);
  if (raw.startsWith("n:")) {
    const n = Number(raw.slice(2));
    return Number.isFinite(n) ? n : raw.slice(2);
  }
  if (raw.startsWith("b:")) return raw.slice(2) === "1";
  if (raw.startsWith("j:")) {
    try {
      return JSON.parse(raw.slice(2)) as unknown;
    } catch {
      return raw.slice(2);
    }
  }
  return raw;
}

/**
 * Aplatit un objet. Les tableaux sont encodés soit élément par élément (`path.0`),
 * soit en feuille `j:[...]` si `arrayAsLeaf` (défaut false).
 */
export function flattenToAttrs(
  input: unknown,
  options?: { skipKeys?: Set<string>; arrayAsLeaf?: boolean },
): AttrPair[] {
  const skip = options?.skipKeys ?? new Set<string>();
  const arrayAsLeaf = options?.arrayAsLeaf ?? false;
  const out: AttrPair[] = [];

  function walk(node: unknown, path: string, depth: number) {
    if (depth > MAX_DEPTH) {
      out.push({ path, value: encodeLeaf(node) });
      return;
    }
    if (Array.isArray(node)) {
      if (arrayAsLeaf || node.some((x) => isPlainObject(x) || Array.isArray(x))) {
        // Tableaux d'objets → indices
        if (!arrayAsLeaf) {
          node.forEach((item, i) => walk(item, path ? `${path}.${i}` : String(i), depth + 1));
          out.push({ path: path ? `${path}.__len` : "__len", value: encodeLeaf(node.length) });
          return;
        }
      }
      out.push({ path, value: encodeLeaf(node) });
      return;
    }
    if (isPlainObject(node)) {
      for (const [k, v] of Object.entries(node)) {
        if (skip.has(k) && !path) continue;
        const next = path ? `${path}.${k}` : k;
        walk(v, next, depth + 1);
      }
      return;
    }
    out.push({ path, value: encodeLeaf(node) });
  }

  walk(input, "", 0);
  return dedupeAttrsByPath(out.filter((a) => a.path !== ""));
}

/**
 * Stockage collection : un gros tableau racine (`__root` / listes JSON historiques)
 * est gardé en une seule feuille pour éviter des milliers de lignes EAV
 * (échecs d’insert, courses delete/insert, perf).
 */
export function flattenCollectionRecord(record: Record<string, unknown>): AttrPair[] {
  const keys = Object.keys(record).filter((k) => k !== "id");
  if (keys.length === 1 && keys[0] === "__root" && Array.isArray(record.__root)) {
    return [{ path: "__root", value: encodeLeaf(record.__root) }];
  }
  return flattenToAttrs(record);
}

/** Dernière valeur gagne — évite les violations de PK sur insert batch. */
export function dedupeAttrsByPath(attrs: AttrPair[]): AttrPair[] {
  const map = new Map<string, string>();
  for (const a of attrs) {
    if (!a.path) continue;
    map.set(a.path, a.value);
  }
  return [...map.entries()].map(([path, value]) => ({ path, value }));
}

/** Reconstruit un objet depuis des attrs. */
export function inflateFromAttrs(attrs: AttrPair[]): Record<string, unknown> {
  const root: Record<string, unknown> = {};

  function setPath(obj: Record<string, unknown>, parts: string[], value: unknown) {
    if (parts.length === 0) return;
    const [head, ...rest] = parts;
    if (rest.length === 0) {
      obj[head] = value;
      return;
    }
    const nextKey = rest[0];
    const asArray = /^\d+$/.test(nextKey) || nextKey === "__len";
    if (asArray && nextKey !== "__len") {
      if (!Array.isArray(obj[head])) obj[head] = [];
      const arr = obj[head] as unknown[];
      const idx = Number(nextKey);
      if (rest.length === 1) {
        arr[idx] = value;
        return;
      }
      if (!arr[idx] || typeof arr[idx] !== "object") arr[idx] = {};
      setPath(arr[idx] as Record<string, unknown>, rest.slice(1), value);
      return;
    }
    if (nextKey === "__len") {
      // longueur informative — ignore à l'inflate si tableau déjà rempli
      if (!Array.isArray(obj[head])) obj[head] = [];
      return;
    }
    if (!isPlainObject(obj[head])) obj[head] = {};
    setPath(obj[head] as Record<string, unknown>, rest, value);
  }

  for (const { path, value } of attrs) {
    if (path.endsWith(".__len")) continue;
    const parts = path.split(".");
    setPath(root, parts, decodeLeaf(value));
  }

  // Compacte les trous éventuels dans les tableaux
  function compact(node: unknown): unknown {
    if (Array.isArray(node)) {
      return node.filter((x) => x !== undefined).map(compact);
    }
    if (isPlainObject(node)) {
      const o: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(node)) o[k] = compact(v);
      return o;
    }
    return node;
  }

  return compact(root) as Record<string, unknown>;
}
