/**
 * Pseudonymisation des personnes CONNUES de l'établissement avant envoi au LLM (Mistral).
 *
 * Principe : les noms des élèves, responsables légaux et personnels sont remplacés par des
 * étiquettes stables (`[Élève 1]`, `[Parent 2]`, `[Personnel 1]`…) dans les textes envoyés au
 * modèle, puis remis tels quels dans sa réponse.
 *
 * Règles de masquage (partie pure, `createPseudonymizer`) :
 *  1. « Prénom Nom », « Nom Prénom », « NOM Prénom » : toujours masqués (insensible à la casse
 *     et aux accents). Étiquette de base, ex. `[Élève 1]`.
 *  2. Nom de famille seul : masqué si (a) il fait au moins 3 lettres (une seule lettre
 *     significative ne suffit jamais), (b) il commence par une majuscule dans le texte,
 *     (c) il n'est pas dans `COMMON_WORD_SURNAMES` (noms qui sont aussi des mots courants :
 *     Petit, Blanc, Roux, Grand, Brun…, ou des mots-outils). Étiquette `[Élève 1 nom]`.
 *     Si plusieurs personnes connues portent ce nom (fratrie, parents) : on retient celle déjà
 *     rencontrée dans la conversation si elle est unique, sinon une étiquette neutre `[Famille N]`.
 *  3. Prénom seul : masqué UNIQUEMENT s'il commence par une majuscule ET désigne, de façon
 *     unique, une personne déjà masquée dans la conversation (via son nom). Sinon laissé tel quel
 *     (« Pierre » dans une phrase n'est pas masqué). Étiquette `[Élève 1 prénom]`.
 *  4. Les formes différentes d'une même personne ont des étiquettes distinctes, pour que la
 *     réinjection redonne exactement le texte d'origine (« Madame Martin » reste « Madame Martin »).
 *  5. Noms composés (Jean-Baptiste, De La Fontaine, Le Gall) : comparés mot par mot ; séparateurs
 *     admis entre les mots : espace, tiret, apostrophe (pas la ponctuation ni le retour à la ligne).
 *
 * Limites assumées : noms inconnus de l'établissement, surnoms, fautes de frappe, noms contenus
 * dans des images/PDF, partie seule d'un nom composé de famille, e-mails/téléphones (non traités).
 */

export type PersonKind = "eleve" | "parent" | "personnel";

export type PseudonymizePerson = {
  firstName?: string | null;
  lastName?: string | null;
  kind: PersonKind;
};

/** Étiquette -> texte d'origine. */
export type PseudonymMap = Record<string, string>;

/**
 * Noms de famille qui sont aussi des mots courants français : jamais masqués seuls
 * (le nom complet « Prénom Petit » l'est toujours). Liste volontairement courte et prudente.
 */
export const COMMON_WORD_SURNAMES: ReadonlySet<string> = new Set([
  "petit", "petite", "grand", "grande", "blanc", "blanche", "roux", "brun", "noir", "gris",
  "vert", "rouge", "bon", "bonne", "fort", "forte", "doux", "jeune", "vieux", "sage", "roy",
  "comte", "duc", "prince", "pape", "moine", "mars", "avril", "mai", "juin", "juillet",
  "noel", "pacques", "jour", "nuit", "rose", "blond", "long", "court", "gras", "gros",
  "dur", "droit", "franc", "libre", "simple", "clair", "claire", "fleur", "force", "vigne",
  "pont", "bois", "marche", "plan", "mont", "fond", "rue", "paix", "joie", "lavie",
  "madame", "monsieur", "mademoiselle", "maitre", "docteur", "classe", "ecole", "college",
  "lycee", "eleve", "parent", "parents", "famille", "madame", "pere", "mere", "fils", "fille",
  "les", "des", "une", "pour", "dans", "avec", "sans", "sous", "sur", "par", "est", "son",
  "ses", "mon", "mes", "ton", "tes", "nos", "vos", "leur", "leurs", "qui", "que", "quoi",
  "dont", "mais", "donc", "car", "plus", "tout", "tous", "bien", "tres", "cher", "chere",
  "bonjour", "merci", "cordialement",
]);

// ---------------------------------------------------------------------------
// Normalisation / tokenisation
// ---------------------------------------------------------------------------

function normToken(s: string): string {
  return s
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/œ/g, "oe")
    .replace(/æ/g, "ae");
}

const WORD_RE = /[\p{L}\p{M}]+/gu;
/** Séparateurs admis à l'intérieur d'un nom composé / entre prénom et nom. */
const GAP_RE = /^[ \t '’\-]+$/;

type Tok = { n: string; start: number; end: number; cap: boolean };

function tokenize(text: string): Tok[] {
  const out: Tok[] = [];
  for (const m of text.matchAll(WORD_RE)) {
    const raw = m[0];
    const first = raw.charAt(0);
    out.push({
      n: normToken(raw),
      start: m.index ?? 0,
      end: (m.index ?? 0) + raw.length,
      cap: first !== first.toLowerCase(),
    });
  }
  return out;
}

function nameTokens(s: string | null | undefined): string[] {
  if (!s) return [];
  return (s.match(WORD_RE) ?? []).map(normToken).filter(Boolean);
}

// ---------------------------------------------------------------------------
// Index de recherche (construit une fois, réutilisable et sans état)
// ---------------------------------------------------------------------------

type Entity = {
  id: number;
  kind: PersonKind;
  key: string;
  firstTokens: string[];
  lastTokens: string[];
};

type PatternType = "full" | "last" | "first";

type TrieNode = {
  next: Map<string, TrieNode>;
  full?: Entity[];
  last?: Entity[];
  first?: Entity[];
};

export type NameIndex = {
  readonly root: TrieNode;
  readonly entityCount: number;
};

const KIND_PRIORITY: Record<PersonKind, number> = { eleve: 0, parent: 1, personnel: 2 };

function addPattern(root: TrieNode, tokens: string[], type: PatternType, e: Entity): void {
  if (!tokens.length) return;
  let node = root;
  for (const t of tokens) {
    let nx = node.next.get(t);
    if (!nx) {
      nx = { next: new Map() };
      node.next.set(t, nx);
    }
    node = nx;
  }
  const list = (node[type] ??= []);
  if (!list.includes(e)) list.push(e);
}

function isDiscriminantLastName(tokens: string[]): boolean {
  if (!tokens.length) return false;
  const joined = tokens.join("");
  if (joined.length < 3) return false;
  if (tokens.length === 1 && COMMON_WORD_SURNAMES.has(tokens[0]!)) return false;
  return true;
}

export function buildNameIndex(people: readonly PseudonymizePerson[]): NameIndex {
  const root: TrieNode = { next: new Map() };
  const byKey = new Map<string, Entity>();
  let nextId = 0;
  for (const p of people) {
    const firstTokens = nameTokens(p.firstName);
    const lastTokens = nameTokens(p.lastName);
    if (!lastTokens.length && !firstTokens.length) continue;
    // Un prénom seul, sans nom, n'est jamais exploitable (trop de faux positifs).
    if (!lastTokens.length) continue;
    const key = `${p.kind}|${firstTokens.join(" ")}|${lastTokens.join(" ")}`;
    let e = byKey.get(key);
    if (!e) {
      e = { id: nextId++, kind: p.kind, key, firstTokens, lastTokens };
      byKey.set(key, e);
      if (firstTokens.length) {
        addPattern(root, [...firstTokens, ...lastTokens], "full", e);
        addPattern(root, [...lastTokens, ...firstTokens], "full", e);
        addPattern(root, firstTokens, "first", e);
      } else {
        addPattern(root, lastTokens, "full", e);
      }
      if (isDiscriminantLastName(lastTokens)) addPattern(root, lastTokens, "last", e);
    }
  }
  return { root, entityCount: byKey.size };
}

// ---------------------------------------------------------------------------
// Pseudonymiseur (avec état de conversation)
// ---------------------------------------------------------------------------

const KIND_LABEL: Record<PersonKind, string> = {
  eleve: "Élève",
  parent: "Parent",
  personnel: "Personnel",
};

type Part = "full" | "nom" | "prenom";

type Match = { start: number; end: number; entities: Entity[]; type: PatternType };

export type Pseudonymizer = {
  mask(text: string): { text: string; map: PseudonymMap };
  unmask(text: string, map?: PseudonymMap): string;
  /** Carte cumulée de toutes les étiquettes émises par ce pseudonymiseur. */
  readonly map: PseudonymMap;
};

/** Plus long motif de l'arbre commençant au jeton `i`, parmi les types demandés. */
function longestMatch(
  root: TrieNode,
  text: string,
  toks: Tok[],
  i: number,
  type: PatternType[],
): Match | null {
  let node: TrieNode = root;
  let best: Match | null = null;
  for (let j = i; j < toks.length; j++) {
    if (j > i && !GAP_RE.test(text.slice(toks[j - 1]!.end, toks[j]!.start))) break;
    const nx = node.next.get(toks[j]!.n);
    if (!nx) break;
    node = nx;
    for (const t of type) {
      const ents = node[t];
      if (ents?.length) best = { start: toks[i]!.start, end: toks[j]!.end, entities: ents, type: t };
    }
  }
  return best;
}

export function createPseudonymizer(
  people: readonly PseudonymizePerson[] | NameIndex,
): Pseudonymizer {
  const index: NameIndex = Array.isArray(people)
    ? buildNameIndex(people as readonly PseudonymizePerson[])
    : (people as NameIndex);
  const root = index.root;

  // État de conversation.
  const entityNumber = new Map<number, number>(); // entité -> numéro dans son type
  const counters: Record<PersonKind, number> = { eleve: 0, parent: 0, personnel: 0 };
  const familyNumber = new Map<string, number>();
  let familyCounter = 0;
  const seenByFirst = new Map<string, Entity[]>(); // prénom normalisé -> entités déjà masquées
  const seenEntities = new Set<Entity>();
  const surfaceLabel = new Map<string, string>(); // `${base}|${surface}` -> étiquette
  const variantCount = new Map<string, number>(); // base -> nb de variantes
  const cumulative: PseudonymMap = {};

  function numberOf(e: Entity): number {
    let n = entityNumber.get(e.id);
    if (!n) {
      n = ++counters[e.kind];
      entityNumber.set(e.id, n);
    }
    return n;
  }

  function markSeen(e: Entity): void {
    if (seenEntities.has(e)) return;
    seenEntities.add(e);
    const k = e.firstTokens.join(" ");
    if (!k) return;
    const l = seenByFirst.get(k) ?? [];
    l.push(e);
    seenByFirst.set(k, l);
  }

  function labelFor(base: string, surface: string): string {
    const memo = surfaceLabel.get(`${base}|${surface}`);
    if (memo) return memo;
    const count = (variantCount.get(base) ?? 0) + 1;
    variantCount.set(base, count);
    const label = count === 1 ? `[${base}]` : `[${base} v${count}]`;
    surfaceLabel.set(`${base}|${surface}`, label);
    cumulative[label] = surface;
    return label;
  }

  function baseName(e: Entity, part: Part): string {
    const b = `${KIND_LABEL[e.kind]} ${numberOf(e)}`;
    return part === "full" ? b : part === "nom" ? `${b} nom` : `${b} prénom`;
  }

  function pickBest(ents: Entity[]): Entity {
    const seen = ents.filter((e) => seenEntities.has(e));
    const pool = seen.length ? seen : ents;
    return [...pool].sort((a, b) => KIND_PRIORITY[a.kind] - KIND_PRIORITY[b.kind] || a.id - b.id)[0]!;
  }

  function mask(text: string): { text: string; map: PseudonymMap } {
    const local: PseudonymMap = {};
    if (!text || index.entityCount === 0) return { text: text ?? "", map: local };
    const toks = tokenize(text);
    if (!toks.length) return { text, map: local };

    const repl: Array<{ start: number; end: number; label: string }> = [];
    const covered = new Array<boolean>(toks.length).fill(false);

    const apply = (m: Match, label: string, firstTok: number, lastTok: number) => {
      repl.push({ start: m.start, end: m.end, label });
      local[label] = text.slice(m.start, m.end);
      for (let k = firstTok; k <= lastTok; k++) covered[k] = true;
    };

    const scan = (types: PatternType[], handle: (m: Match, i: number, last: number) => void) => {
      let i = 0;
      while (i < toks.length) {
        if (covered[i]) {
          i++;
          continue;
        }
        const m = longestMatch(root, text, toks, i, types);
        if (!m) {
          i++;
          continue;
        }
        let last = i;
        while (last + 1 < toks.length && toks[last + 1]!.end <= m.end) last++;
        if (covered.slice(i, last + 1).some(Boolean)) {
          i++;
          continue;
        }
        handle(m, i, last);
        i = covered[i] ? last + 1 : i + 1;
      }
    };

    // Passe 1 : noms complets.
    scan(["full"], (m, i, last) => {
      const e = pickBest(m.entities);
      markSeen(e);
      const surface = text.slice(m.start, m.end);
      apply(m, labelFor(baseName(e, "full"), surface), i, last);
    });

    // Passe 2 : nom de famille seul (majuscule initiale obligatoire).
    scan(["last"], (m, i, last) => {
      if (!toks[i]!.cap) return;
      const seen = m.entities.filter((e) => seenEntities.has(e));
      const surface = text.slice(m.start, m.end);
      let label: string;
      if (m.entities.length === 1) {
        markSeen(m.entities[0]!);
        label = labelFor(baseName(m.entities[0]!, "nom"), surface);
      } else if (seen.length === 1) {
        label = labelFor(baseName(seen[0]!, "nom"), surface);
      } else {
        const fk = m.entities[0]!.lastTokens.join(" ");
        let n = familyNumber.get(fk);
        if (!n) {
          n = ++familyCounter;
          familyNumber.set(fk, n);
        }
        label = labelFor(`Famille ${n}`, surface);
      }
      apply(m, label, i, last);
    });

    // Passe 3 : prénom seul, uniquement s'il désigne une seule personne déjà masquée.
    scan(["first"], (m, i, last) => {
      if (!toks[i]!.cap) return;
      const key = toks
        .slice(i, last + 1)
        .map((t) => t.n)
        .join(" ");
      const seen = seenByFirst.get(key);
      if (!seen || seen.length !== 1) return;
      const surface = text.slice(m.start, m.end);
      apply(m, labelFor(baseName(seen[0]!, "prenom"), surface), i, last);
    });

    if (!repl.length) return { text, map: local };
    repl.sort((a, b) => a.start - b.start);
    let out = "";
    let pos = 0;
    for (const r of repl) {
      out += text.slice(pos, r.start) + r.label;
      pos = r.end;
    }
    out += text.slice(pos);
    return { text: out, map: local };
  }

  return {
    mask,
    unmask: (text, map) => unmaskText(text, map ?? cumulative),
    get map() {
      return cumulative;
    },
  };
}

// ---------------------------------------------------------------------------
// Démasquage tolérant
// ---------------------------------------------------------------------------

const LABEL_RE =
  /\[\s*(élève|eleve|parent|personnel|famille)\s*(\d+)\s*(nom|prénom|prenom)?\s*(?:v\s*(\d+))?\s*\]/giu;

const KIND_CANON: Record<string, string> = {
  "élève": "Élève",
  eleve: "Élève",
  parent: "Parent",
  personnel: "Personnel",
  famille: "Famille",
};

function canonLabel(
  kind: string,
  num: string,
  part: string | undefined,
  variant: string | undefined,
): string {
  const k = KIND_CANON[kind.toLowerCase()] ?? kind;
  const p = part ? (part.toLowerCase().startsWith("pr") ? " prénom" : " nom") : "";
  const v = variant && variant !== "1" ? ` v${variant}` : "";
  return `[${k} ${Number(num)}${p}${v}]`;
}

/** Remet les vrais noms ; les étiquettes inconnues (hallucinées par le modèle) sont laissées. */
export function unmaskText(text: string, map: PseudonymMap): string {
  if (!text || !map) return text;
  return text.replace(LABEL_RE, (whole, kind, num, part, variant) => {
    const original = map[canonLabel(kind, num, part, variant)];
    return original ?? whole;
  });
}

// ---------------------------------------------------------------------------
// Partie serveur : chargement des personnes connues de l'établissement
// ---------------------------------------------------------------------------

const CACHE_TTL_MS = 10 * 60 * 1000;
const CACHE_MAX_ETABLISSEMENTS = 50;
const indexCache = new Map<string, { at: number; index: NameIndex }>();

/**
 * Élèves actifs (listElevesFromDb), personnels actifs (listPersonnelFromDb) et responsables
 * légaux (table foyer_responsable : aucune fonction de liste existante, seule la lecture
 * prénom + nom est faite ici). Uniquement prénom/nom (minimisation). Ne lève jamais.
 */
export async function loadEstablishmentPeople(
  etablissementId: string,
): Promise<PseudonymizePerson[]> {
  const people: PseudonymizePerson[] = [];
  try {
    const [{ listElevesFromDb, listPersonnelFromDb }, { getDb, isDatabaseConfigured }] =
      await Promise.all([import("@/app/lib/ent-core-db"), import("@/db/index")]);
    if (!isDatabaseConfigured()) return [];
    const [eleves, personnel, responsables] = await Promise.all([
      listElevesFromDb(etablissementId, { actifsPourListes: true }).catch(() => []),
      listPersonnelFromDb(etablissementId).catch(() => []),
      (async () => {
        const [{ foyerResponsable }, { eq }] = await Promise.all([
          import("@/db/schema"),
          import("drizzle-orm"),
        ]);
        return getDb()
          .select({ nom: foyerResponsable.nom, prenom: foyerResponsable.prenom })
          .from(foyerResponsable)
          .where(eq(foyerResponsable.etablissementId, etablissementId));
      })().catch(() => []),
    ]);
    for (const e of eleves) people.push({ kind: "eleve", firstName: e.prenom, lastName: e.nom });
    for (const r of responsables)
      people.push({ kind: "parent", firstName: r.prenom, lastName: r.nom });
    for (const p of personnel)
      people.push({ kind: "personnel", firstName: p.firstName, lastName: p.lastName });
  } catch (err) {
    // Aucune donnée personnelle dans le log : seulement le type d'erreur.
    console.warn("[pseudonymize] chargement des personnes impossible:", (err as Error)?.name);
    return [];
  }
  return people;
}

async function getIndex(etablissementId: string): Promise<NameIndex> {
  const now = Date.now();
  const hit = indexCache.get(etablissementId);
  if (hit && now - hit.at < CACHE_TTL_MS) return hit.index;
  const index = buildNameIndex(await loadEstablishmentPeople(etablissementId));
  // On ne met pas en cache un résultat vide (erreur transitoire possible).
  if (index.entityCount > 0) {
    if (indexCache.size >= CACHE_MAX_ETABLISSEMENTS) {
      const oldest = indexCache.keys().next().value;
      if (oldest !== undefined) indexCache.delete(oldest);
    }
    indexCache.set(etablissementId, { at: now, index });
  }
  return index;
}

/**
 * Variante stricte (messagerie) : lève si l'index des noms est vide ou indisponible,
 * au lieu de laisser partir le texte en clair. Un établissement a toujours du personnel :
 * un index vide signifie un échec de chargement.
 */
export async function pseudonymizeForLlmStrict(
  etablissementId: string,
  texts: string[],
): Promise<{ texts: string[]; restore: (answer: string) => string }> {
  if (!etablissementId) throw new Error("etablissement manquant");
  const index = await getIndex(etablissementId);
  if (index.entityCount === 0) throw new Error("index des noms indisponible");
  const p = createPseudonymizer(index);
  return { texts: texts.map((t) => p.mask(t).text), restore: (answer) => p.unmask(answer) };
}

/** Vide le cache (tests, ou après un gros import d'élèves). */
export function clearPseudonymizeCache(etablissementId?: string): void {
  if (etablissementId) indexCache.delete(etablissementId);
  else indexCache.clear();
}

/**
 * Point d'intégration : masque tous les textes envoyés au LLM avec une numérotation commune,
 * et fournit `restore` pour remettre les vrais noms dans la réponse. Ne lève jamais :
 * en cas d'échec, les textes sont renvoyés tels quels et `restore` est l'identité.
 */
export async function pseudonymizeForLlm(
  etablissementId: string,
  texts: string[],
): Promise<{ texts: string[]; restore: (answer: string) => string; maskedCount: number }> {
  try {
    if (!etablissementId) throw new Error("etablissement manquant");
    const index = await getIndex(etablissementId);
    if (index.entityCount === 0) return { texts, restore: (a) => a, maskedCount: 0 };
    const p = createPseudonymizer(index);
    const masked = texts.map((t) => p.mask(t).text);
    return {
      texts: masked,
      restore: (answer) => p.unmask(answer),
      maskedCount: Object.keys(p.map).length,
    };
  } catch (err) {
    console.warn("[pseudonymize] désactivée pour cette requête:", (err as Error)?.name);
    return { texts, restore: (a) => a, maskedCount: 0 };
  }
}
