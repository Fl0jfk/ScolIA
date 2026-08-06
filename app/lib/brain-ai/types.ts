export type BrainAudience = "public" | "private";

export type BrainChoiceOption = {
  value: string;
  label: string;
};

export type BrainPendingChoices = {
  tool: string;
  field: string;
  promptFr: string;
  options: BrainChoiceOption[];
  draftArgs: Record<string, unknown>;
  /** single = liste déroulante ; multi = cases à cocher ; date = sélecteur de date ; text = saisie libre */
  selectionType?: "single" | "multi" | "date" | "text";
};

export type BrainToolResult =
  | { ok: true; data: unknown; summaryFr?: string }
  | { ok: false; error: string; code?: string }
  | {
      ok: false;
      needsConfirmation: true;
      tool: string;
      args: Record<string, unknown>;
      summaryFr: string;
    }
  | {
      ok: false;
      needsChoices: true;
      tool: string;
      field: string;
      promptFr: string;
      options: BrainChoiceOption[];
      draftArgs: Record<string, unknown>;
      selectionType?: "single" | "multi" | "date" | "text";
    };

export type BrainToolCtx = {
  userId: string | null;
  roles: string[];
  isOrgAdmin: boolean;
  audience: BrainAudience;
  confirmed: boolean;
  firstName?: string;
  lastName?: string;
  email?: string;
  phone?: string;
};

export type BrainPendingConfirmation = {
  tool: string;
  args: Record<string, unknown>;
  summaryFr: string;
};

export type BrainConversationState = {
  conversationId: string;
  intent?: string;
  slots: Record<string, unknown>;
  pendingConfirmation?: BrainPendingConfirmation | null;
  pendingChoices?: BrainPendingChoices | null;
};

export type BrainCta = {
  label: string;
  href: string;
};

export type BrainChatResponse = {
  answer: string;
  domain?: string;
  usedFile?: string;
  usedDomains?: Array<{ id: string; file: string; label: string }>;
  fallbackFrom?: string;
  conversationState?: BrainConversationState;
  pendingConfirmation?: BrainPendingConfirmation | null;
  pendingChoices?: BrainPendingChoices | null;
  ctas?: BrainCta[];
};

export type BrainToolDefinition = {
  name: string;
  description: string;
  parameters: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
    additionalProperties?: boolean;
  };
  /** Préfixe path intranet pour le gate (ex. /prof-room). */
  pathPrefix?: string;
  /** Module id pour logs / filtre. */
  moduleId?: string;
  /** Exige une session Clerk. */
  requiresAuth: boolean;
  mutates: boolean;
  handler: (ctx: BrainToolCtx, args: Record<string, unknown>) => Promise<BrainToolResult>;
};
