import type { BrainConversationState, BrainPendingConfirmation } from "@/app/lib/brain-ai/types";

export function createConversationState(conversationId?: string): BrainConversationState {
  return {
    conversationId: conversationId || `c_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`,
    slots: {},
    pendingConfirmation: null,
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
  let pendingConfirmation: BrainPendingConfirmation | null = null;
  const p = o.pendingConfirmation;
  if (p && typeof p === "object") {
    const pc = p as Record<string, unknown>;
    if (typeof pc.tool === "string" && pc.args && typeof pc.args === "object") {
      pendingConfirmation = {
        tool: pc.tool,
        args: pc.args as Record<string, unknown>,
        summaryFr: typeof pc.summaryFr === "string" ? pc.summaryFr : "Confirmer cette action ?",
      };
    }
  }
  return {
    conversationId,
    intent: typeof o.intent === "string" ? o.intent : undefined,
    slots,
    pendingConfirmation,
  };
}

export function withPendingConfirmation(
  state: BrainConversationState,
  pending: BrainPendingConfirmation | null,
): BrainConversationState {
  return { ...state, pendingConfirmation: pending };
}

export function mergeSlots(
  state: BrainConversationState,
  patch: Record<string, unknown>,
): BrainConversationState {
  return { ...state, slots: { ...state.slots, ...patch } };
}
