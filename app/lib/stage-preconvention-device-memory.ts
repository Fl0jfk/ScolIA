/**
 * Mémoire appareil (localStorage) pour la préconvention publique.
 * Après OTP, on conserve une preuve serveur (`identityProof`) pour éviter
 * de renvoyer un code à chaque visite tant qu'elle est valide.
 * Plusieurs enfants (fratrie, jumeaux) peuvent être mémorisés sur un même appareil ;
 * chaque preuve est propre à un seul enfant.
 */

export type StagePreconventionDeviceMemory = {
  version: 3;
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  /** Preuve post-OTP délivrée par le serveur (TTL côté serveur). */
  identityProof?: string;
  savedAt: string;
};

export type StagePreconventionDeviceMemoryInput = {
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  identityProof?: string;
};

const STORAGE_KEY = "scola.stages.preconvention.identity.v3";
const V2_STORAGE_KEY = "scola.stages.preconvention.identity.v2";
const LEGACY_STORAGE_KEY = "scola.stages.preconvention.identity.v1";
/** Conservé ~6 mois sur l'appareil (année scolaire typique). */
export const PRECONVENTION_MEMORY_MAX_AGE_MS = 180 * 24 * 60 * 60 * 1000;
export const PRECONVENTION_MEMORY_MAX_CHILDREN = 6;

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function norm(value: string | undefined | null): string {
  return String(value ?? "").trim().toLowerCase();
}

function sameChild(
  a: { nom: string; prenom: string; dateNaissance: string },
  b: { nom: string; prenom: string; dateNaissance: string },
): boolean {
  return (
    norm(a.nom) === norm(b.nom) &&
    norm(a.prenom) === norm(b.prenom) &&
    norm(a.dateNaissance) === norm(b.dateNaissance)
  );
}

/** Normalise une entrée brute (v2 ou v3) ; null si invalide ou expirée. */
function parseEntry(
  raw: unknown,
  version: 2 | 3,
  now: number,
): StagePreconventionDeviceMemory | null {
  if (!raw || typeof raw !== "object") return null;
  const parsed = raw as Record<string, unknown>;
  if (parsed.version !== version) return null;
  const nom = String(parsed.nom ?? "").trim();
  const prenom = String(parsed.prenom ?? "").trim();
  const dateNaissance = String(parsed.dateNaissance ?? "").trim();
  if (!nom || !prenom || !dateNaissance) return null;
  const savedAt = String(parsed.savedAt ?? "");
  const savedMs = Date.parse(savedAt);
  if (!Number.isFinite(savedMs) || now - savedMs > PRECONVENTION_MEMORY_MAX_AGE_MS) return null;
  const classe = String(parsed.classe ?? "").trim() || undefined;
  const identityProof = String(parsed.identityProof ?? "").trim() || undefined;
  return { version: 3, nom, prenom, dateNaissance, classe, identityProof, savedAt };
}

/** Plus récent d'abord, dédupliqué (la première occurrence gagne), limité à 6. */
function normalizeList(
  entries: StagePreconventionDeviceMemory[],
): StagePreconventionDeviceMemory[] {
  const sorted = [...entries].sort((a, b) => Date.parse(b.savedAt) - Date.parse(a.savedAt));
  const out: StagePreconventionDeviceMemory[] = [];
  for (const entry of sorted) {
    if (out.some((e) => sameChild(e, entry))) continue;
    out.push(entry);
    if (out.length >= PRECONVENTION_MEMORY_MAX_CHILDREN) break;
  }
  return out;
}

/** Parse le contenu brut de la clé v3 (liste JSON). Pur, sans `window`. */
export function parsePreconventionMemoryList(
  raw: string | null,
  now: number = Date.now(),
): StagePreconventionDeviceMemory[] {
  if (!raw) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const entries = parsed
      .map((item) => parseEntry(item, 3, now))
      .filter((e): e is StagePreconventionDeviceMemory => e !== null);
    return normalizeList(entries);
  } catch {
    return [];
  }
}

/** Convertit le contenu brut de l'ancienne clé v2 (un seul enfant) en liste. Pur. */
export function migratePreconventionMemoryV2(
  raw: string | null,
  now: number = Date.now(),
): StagePreconventionDeviceMemory[] {
  if (!raw) return [];
  try {
    const entry = parseEntry(JSON.parse(raw), 2, now);
    return entry ? [entry] : [];
  } catch {
    return [];
  }
}

/** Ajoute ou remplace l'enfant (mis en tête), puis applique la limite. Pur. */
export function upsertPreconventionMemory(
  list: StagePreconventionDeviceMemory[],
  input: StagePreconventionDeviceMemoryInput,
  now: number = Date.now(),
): StagePreconventionDeviceMemory[] {
  const nom = input.nom.trim();
  const prenom = input.prenom.trim();
  const dateNaissance = input.dateNaissance.trim();
  if (!nom || !prenom || !dateNaissance) return list;
  const entry: StagePreconventionDeviceMemory = {
    version: 3,
    nom,
    prenom,
    dateNaissance,
    classe: input.classe?.trim() || undefined,
    identityProof: input.identityProof?.trim() || undefined,
    savedAt: new Date(now).toISOString(),
  };
  const rest = list.filter((e) => !sameChild(e, entry));
  return [entry, ...rest].slice(0, PRECONVENTION_MEMORY_MAX_CHILDREN);
}

/** Retire un enfant de la liste. Pur. */
export function removePreconventionMemory(
  list: StagePreconventionDeviceMemory[],
  child: { nom: string; prenom: string; dateNaissance: string },
): StagePreconventionDeviceMemory[] {
  return list.filter((e) => !sameChild(e, child));
}

/**
 * Cherche un enfant par nom + prénom (+ date de naissance si fournie). Pur.
 * Sans date, renvoie l'entrée seulement si elle est non ambiguë.
 */
export function findPreconventionMemoryIn(
  list: StagePreconventionDeviceMemory[],
  query: { nom: string; prenom: string; dateNaissance?: string },
): StagePreconventionDeviceMemory | null {
  const matches = list.filter(
    (e) =>
      norm(e.nom) === norm(query.nom) &&
      norm(e.prenom) === norm(query.prenom) &&
      (!norm(query.dateNaissance) || norm(e.dateNaissance) === norm(query.dateNaissance)),
  );
  return matches.length === 1 ? matches[0] : null;
}

function persist(list: StagePreconventionDeviceMemory[]): void {
  try {
    if (list.length === 0) {
      window.localStorage.removeItem(STORAGE_KEY);
    } else {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(list));
    }
  } catch {
    // Quota / mode privé : ignorer silencieusement.
  }
}

export function readPreconventionDeviceMemories(): StagePreconventionDeviceMemory[] {
  if (!isBrowser()) return [];
  try {
    try {
      // Ancienne clé sans preuve → on nettoie (OTP désormais obligatoire).
      window.localStorage.removeItem(LEGACY_STORAGE_KEY);
    } catch {
      // ignore
    }

    let list = parsePreconventionMemoryList(window.localStorage.getItem(STORAGE_KEY));

    // Migration v2 (un seul enfant) → liste v3, puis suppression de v2.
    const v2Raw = window.localStorage.getItem(V2_STORAGE_KEY);
    if (v2Raw !== null) {
      const migrated = migratePreconventionMemoryV2(v2Raw);
      if (migrated.length > 0) {
        list = normalizeList([...list, ...migrated]);
        persist(list);
      }
      window.localStorage.removeItem(V2_STORAGE_KEY);
    }
    return list;
  } catch {
    return [];
  }
}

export function findPreconventionDeviceMemory(query: {
  nom: string;
  prenom: string;
  dateNaissance?: string;
}): StagePreconventionDeviceMemory | null {
  return findPreconventionMemoryIn(readPreconventionDeviceMemories(), query);
}

export function writePreconventionDeviceMemory(input: StagePreconventionDeviceMemoryInput): void {
  if (!isBrowser()) return;
  try {
    const list = readPreconventionDeviceMemories();
    persist(upsertPreconventionMemory(list, input));
  } catch {
    // ignore
  }
}

export function forgetPreconventionDeviceMemory(child: {
  nom: string;
  prenom: string;
  dateNaissance: string;
}): void {
  if (!isBrowser()) return;
  try {
    persist(removePreconventionMemory(readPreconventionDeviceMemories(), child));
  } catch {
    // ignore
  }
}

export function clearPreconventionDeviceMemory(): void {
  if (!isBrowser()) return;
  try {
    window.localStorage.removeItem(STORAGE_KEY);
    window.localStorage.removeItem(V2_STORAGE_KEY);
    window.localStorage.removeItem(LEGACY_STORAGE_KEY);
  } catch {
    // ignore
  }
}
