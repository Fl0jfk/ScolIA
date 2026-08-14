export type RequestRouteDef = {
  id: string;
  category: string;
  roleLabel: string;
  leaderEmails: () => string[];
  executorEmails: () => string[];
  primaryEmail: () => string;
  ccEmails: () => string[];
  poolEmails: () => string[];
  promptLine: string;
  keywords: string[];
};

type ResolvedRequestRouting = {
  category: string;
  assignedTo: {
    routeId: string;
    unit: string;
    roleLabel: string;
    email: string;
    claimedBy: null | { email: string; name?: string };
    poolEmails?: string[];
    ccEmails?: string[];
  };
  source: "ai" | "fallback" | "manual";
  confidence: number;
  reason: string;
  suggestedRouteId?: string;
  routingMeta?: { assignmentId: string; taskId: string };
  directionHint?: {
    routeId: string;
    roleLabel: string;
    email: string;
  };
};
