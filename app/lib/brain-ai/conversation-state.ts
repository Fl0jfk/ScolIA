import type {
  BrainConversationState,
  BrainPendingChoices,
  BrainPendingConfirmation,
} from "@/app/lib/brain-ai/types";

export function createConversationState(conversationId?: string): BrainConversationState {
  return {
    conversationId: conversationId || `c_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`,
    slots: {},
    pendingConfirmation: null,
    pendingChoices: null,
  };
}

function parsePendingConfirmation(raw: unknown): BrainPendingConfirmation | null {
  if (!raw || typeof raw !== "object") return null;
  const pc = raw as Record<string, unknown>;
  if (typeof pc.tool !== "string" || !pc.args || typeof pc.args !== "object") return null;
  return {
    tool: pc.tool,
    args: pc.args as Record<string, unknown>,
    summaryFr: typeof pc.summaryFr === "string" ? pc.summaryFr : "Confirmer cette action ?",
  };
}

function parsePendingChoices(raw: unknown): BrainPendingChoices | null {
  if (!raw || typeof raw !== "object") return null;
  const pc = raw as Record<string, unknown>;
  if (typeof pc.tool !== "string" || typeof pc.field !== "string") return null;
  if (!pc.draftArgs || typeof pc.draftArgs !== "object") return null;
  const options = Array.isArray(pc.options)
    ? pc.options
        .filter((o): o is Record<string, unknown> => Boolean(o && typeof o === "object"))
        .map((o) => ({
          value: String(o.value ?? ""),
          label: String(o.label ?? o.value ?? ""),
        }))
        .filter((o) => o.value)
    : [];
  const selectionType =
    pc.selectionType === "multi" ||
    pc.selectionType === "date" ||
    pc.selectionType === "text" ||
    pc.selectionType === "single"
      ? pc.selectionType
      : "single";
  return {
    tool: pc.tool,
    field: pc.field,
    promptFr: typeof pc.promptFr === "string" ? pc.promptFr : "Faites un choix :",
    options,
    draftArgs: pc.draftArgs as Record<string, unknown>,
    selectionType,
  };
}

export function normalizeConversationState(raw: unknown): BrainConversationState {
  if (!raw || typeof raw !== "object") return createConversationState();
  const o = raw as Record<string, unknown>;
  const conversationId =
    typeof o.conversationId === "string" && o.conversationId.trim()
      ? o.conversationId.trim()
      : createConversationState().conversationId;
  const slots =
    o.slots && typeof o.slots === "object" && !Array.isArray(o.slots)
      ? (o.slots as Record<string, unknown>)
      : {};
  return {
    conversationId,
    intent: typeof o.intent === "string" ? o.intent : undefined,
    slots,
    pendingConfirmation: parsePendingConfirmation(o.pendingConfirmation),
    pendingChoices: parsePendingChoices(o.pendingChoices),
  };
}

export function withPendingConfirmation(
  state: BrainConversationState,
  pending: BrainPendingConfirmation | null,
): BrainConversationState {
  return { ...state, pendingConfirmation: pending, pendingChoices: pending ? null : state.pendingChoices };
}

export function withPendingChoices(
  state: BrainConversationState,
  pending: BrainPendingChoices | null,
): BrainConversationState {
  return { ...state, pendingChoices: pending, pendingConfirmation: pending ? null : state.pendingConfirmation };
}

function mergeSlots(
  state: BrainConversationState,
  patch: Record<string, unknown>,
): BrainConversationState {
  return { ...state, slots: { ...state.slots, ...patch } };
}
