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

/** Demande un dépôt de fichier (ex. PDF photocopie) avant de poursuivre. */
export type BrainPendingFileUpload = {
  tool: string;
  promptFr: string;
  draftArgs: Record<string, unknown>;
  /** Si true, l’utilisateur peut continuer sans fichier. */
  optional?: boolean;
  accept?: string;
};

/** Action UI exécutée côté navigateur après la réponse Brain. */
export type BrainClientAction =
  | { type: "open_route"; href: string; label?: string }
  | { type: "open_eleve_dossier"; eleveId: string; subView?: "dossier" | "inscription" }
  | { type: "open_url_modal"; href: string; title?: string };

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
    }
  | {
      ok: false;
      needsFileUpload: true;
      tool: string;
      promptFr: string;
      draftArgs: Record<string, unknown>;
      optional?: boolean;
      accept?: string;
    };

export type BrainToolCtx = {
  userId: string | null;
  roles: string[];
  isOrgAdmin: boolean;
  audience: BrainAudience;
  confirmed: boolean;
  firstName?: string;
  lastName?: string;
  /** Libellé complet Better-Auth — repli si first/last absents. */
  name?: string;
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
  pendingFileUpload?: BrainPendingFileUpload | null;
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
  pendingFileUpload?: BrainPendingFileUpload | null;
  ctas?: BrainCta[];
  clientActions?: BrainClientAction[];
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
  /** Exige une session authentifiée. */
  requiresAuth: boolean;
  mutates: boolean;
  handler: (ctx: BrainToolCtx, args: Record<string, unknown>) => Promise<BrainToolResult>;
};
