/** Clés / helpers mémoire ScolIA (localStorage, côté client). */

export const SCOLIA_AI_NAME = "ScolIA";
const SCOLIA_AI_MEMORY_KEY = "scolia-ai-memory-v1";
export const SCOLIA_AI_PAGE_PATH = "/scolia-ai";

export type ScoliaMemoryCta = {
  label: string;
  href: string;
  /** Aperçu PDF (PAP/PAI…) via modale, pas une navigation. */
  preview?: boolean;
  subtitle?: string;
  group?: string;
};

export type ScoliaMemoryDocCatalogItem = {
  title: string;
  subtitle?: string;
  href: string;
  preview?: boolean;
  dossierHref?: string;
  ext?: string;
};

export type ScoliaMemoryDocCatalogGroup = {
  title: string;
  count: number;
  items: ScoliaMemoryDocCatalogItem[];
};

export type ScoliaMemoryDocCatalog = {
  title: string;
  kindLabel?: string;
  total: number;
  groups: ScoliaMemoryDocCatalogGroup[];
};

export type ScoliaMemoryMessage = {
  role: "user" | "assistant";
  content: string;
  /** Actions / pièces rattachées au tour (persistées avec l’historique). */
  ctas?: ScoliaMemoryCta[];
  /** Catalogue documents groupé (PAP par classe, etc.). */
  docCatalog?: ScoliaMemoryDocCatalog;
};

type ScoliaMemorySnapshot = {
  version: 1;
  updatedAt: string;
  messages: ScoliaMemoryMessage[];
  conversationState: Record<string, unknown> | null;
  pendingConfirmation: {
    tool: string;
    args: Record<string, unknown>;
    summaryFr: string;
  } | null;
  pendingChoices?: {
    tool: string;
    field: string;
    promptFr: string;
    options: Array<{ value: string; label: string }>;
    draftArgs: Record<string, unknown>;
    selectionType?: "single" | "multi" | "date" | "text";
  } | null;
};

export function defaultWelcomeMessage(): ScoliaMemoryMessage {
  return {
    role: "assistant",
    content:
      `Bonjour, je suis ${SCOLIA_AI_NAME}. Parlez au micro ou écrivez — j’ouvre les pages (dossiers élèves, sorties, salles…) et j’agis avec votre validation.`,
  };
}

export function loadScoliaMemory(): ScoliaMemorySnapshot | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = localStorage.getItem(SCOLIA_AI_MEMORY_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as ScoliaMemorySnapshot;
    if (!parsed || parsed.version !== 1 || !Array.isArray(parsed.messages)) return null;
    return parsed;
  } catch {
    return null;
  }
}

export function saveScoliaMemory(snapshot: Omit<ScoliaMemorySnapshot, "version" | "updatedAt">): void {
  if (typeof window === "undefined") return;
  try {
    const payload: ScoliaMemorySnapshot = {
      version: 1,
      updatedAt: new Date().toISOString(),
      messages: snapshot.messages.slice(-80),
      conversationState: snapshot.conversationState,
      pendingConfirmation: snapshot.pendingConfirmation,
      pendingChoices: snapshot.pendingChoices ?? null,
    };
    localStorage.setItem(SCOLIA_AI_MEMORY_KEY, JSON.stringify(payload));
  } catch {
    /* quota / private mode */
  }
}

export function clearScoliaMemory(): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.removeItem(SCOLIA_AI_MEMORY_KEY);
  } catch {
    /* ignore */
  }
}
