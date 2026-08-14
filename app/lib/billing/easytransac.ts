import "server-only";

import { platformAppOrigin } from "@/app/lib/platform-portal-url";

type EasytransacEnvironment = "sandbox" | "production";

export const EASYTRANSAC_BILLING = {
  isConfigured: Boolean(process.env.EASYTRANSAC_API_KEY?.trim()),
  apiKey: process.env.EASYTRANSAC_API_KEY?.trim() ?? "",
  environment: (process.env.EASYTRANSAC_ENV?.trim() === "production"
    ? "production"
    : "sandbox") as EasytransacEnvironment,
  webhookSecret: process.env.EASYTRANSAC_WEBHOOK_SECRET?.trim() ?? "",
};

function apiBase(): string {
  return EASYTRANSAC_BILLING.environment === "production"
    ? "https://api.easytransac.com/v2"
    : "https://api.sandbox.easytransac.com/v2";
}

let tokenCache: { token: string; expiresAt: number } | null = null;

async function getAccessToken(): Promise<string> {
  if (!EASYTRANSAC_BILLING.isConfigured) {
    throw new Error("Easytransac non configuré (EASYTRANSAC_API_KEY).");
  }
  if (tokenCache && tokenCache.expiresAt > Date.now() + 30_000) {
    return tokenCache.token;
  }
  const res = await fetch(`${apiBase()}/auth`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ apiKey: EASYTRANSAC_BILLING.apiKey }),
  });
  const json = (await res.json()) as {
    success?: boolean;
    data?: { token?: string; expiresIn?: number };
    detail?: string;
  };
  if (!res.ok || !json.success || !json.data?.token) {
    throw new Error(json.detail || "Authentification Easytransac impossible.");
  }
  tokenCache = {
    token: json.data.token,
    expiresAt: Date.now() + (json.data.expiresIn ?? 3600) * 1000,
  };
  return tokenCache.token;
}

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const token = await getAccessToken();
  const res = await fetch(`${apiBase()}${path}`, {
    ...init,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
      ...(init?.headers || {}),
    },
  });
  const json = (await res.json()) as T & { success?: boolean; detail?: string };
  if (!res.ok || json.success === false) {
    throw new Error((json as { detail?: string }).detail || `Easytransac ${path} (${res.status})`);
  }
  return json;
}

type EasytransacInitPaymentInput = {
  amountCents: number;
  email: string;
  firstName: string;
  lastName: string;
  orderId: string;
  description: string;
  returnUrl: string;
  address?: string;
  zipCode?: string;
  city?: string;
};

type EasytransacInitPaymentResult = {
  tid: string;
  redirectUrl: string;
  status: string;
};

export async function initEasytransacOpenBankingPayment(
  input: EasytransacInitPaymentInput,
): Promise<EasytransacInitPaymentResult> {
  const body = {
    amount: Math.round(input.amountCents),
    email: input.email,
    returnUrl: input.returnUrl,
    returnMethod: "GET" as const,
    orderId: input.orderId,
    description: input.description,
    firstname: input.firstName.replace(/[^A-Za-z0-9]/g, "") || "Admin",
    lastname: input.lastName.replace(/[^A-Za-z0-9]/g, "") || "Scola",
    address: (input.address || "Adresse").replace(/[^A-Za-z0-9 ]/g, "").slice(0, 128),
    zipCode: (input.zipCode || "00000").replace(/[^A-Za-z0-9]/g, "").slice(0, 16),
    city: (input.city || "Ville").replace(/[^A-Za-z0-9 ]/g, "").slice(0, 64),
    country: "FRA",
    language: "FRE",
  };

  const json = await apiFetch<{
    data: { tid: string; redirectUrl: string; status: string };
  }>("/open-banking/init", { method: "POST", body: JSON.stringify(body) });

  if (!json.data?.redirectUrl || !json.data.tid) {
    throw new Error("Réponse Easytransac incomplète.");
  }
  return {
    tid: json.data.tid,
    redirectUrl: json.data.redirectUrl,
    status: json.data.status,
  };
}

export async function getEasytransacTransactionStatus(opts: {
  tid?: string;
  orderId?: string;
}): Promise<{ status: string; tid: string; amount: number }> {
  const q = new URLSearchParams();
  if (opts.tid) q.set("tid", opts.tid);
  if (opts.orderId) q.set("orderId", opts.orderId);
  const json = await apiFetch<{
    data: { status: string; tid: string; amount: number };
  }>(`/transaction/status?${q.toString()}`);
  return json.data;
}

export function easytransacReturnUrl(signupId: string, token: string): string {
  const base = platformAppOrigin();
  return `${base}/api/billing/easytransac/return?signupId=${encodeURIComponent(signupId)}&token=${encodeURIComponent(token)}`;
}

/** URL à coller dans EasyTransac → Application → URL de notification. */
function easytransacNotificationUrl(): string {
  return `${platformAppOrigin()}/api/billing/easytransac/webhook`;
}

export function isEasytransacPaymentSuccess(status: string): boolean {
  return status === "captured" || status === "authorized";
}

export function isEasytransacPaymentFailed(status: string): boolean {
  const s = status.toLowerCase();
  return (
    s === "failed" ||
    s === "refused" ||
    s === "cancelled" ||
    s === "canceled" ||
    s === "error" ||
    s === "declined"
  );
}

type EasytransacSddPaymentInput = EasytransacInitPaymentInput & {
  clientId?: string;
};

/** Prélèvement SEPA (SDD) — abonnement récurrent. */
export async function initEasytransacSddPayment(
  input: EasytransacSddPaymentInput,
): Promise<EasytransacInitPaymentResult> {
  const body = {
    amount: Math.round(input.amountCents),
    email: input.email,
    returnUrl: input.returnUrl,
    returnMethod: "GET" as const,
    orderId: input.orderId,
    description: input.description,
    firstname: input.firstName.replace(/[^A-Za-z0-9]/g, "") || "Admin",
    lastname: input.lastName.replace(/[^A-Za-z0-9]/g, "") || "Scola",
    address: (input.address || "Adresse").replace(/[^A-Za-z0-9 ]/g, "").slice(0, 128),
    zipCode: (input.zipCode || "00000").replace(/[^A-Za-z0-9]/g, "").slice(0, 16),
    city: (input.city || "Ville").replace(/[^A-Za-z0-9 ]/g, "").slice(0, 64),
    country: "FRA",
    language: "FRE",
    ...(input.clientId ? { clientId: input.clientId } : {}),
  };

  const json = await apiFetch<{
    data: { tid: string; redirectUrl?: string; status: string; clientId?: string };
  }>("/payment/sdd/init", { method: "POST", body: JSON.stringify(body) });

  if (!json.data?.tid) {
    throw new Error("Réponse Easytransac SDD incomplète.");
  }
  return {
    tid: json.data.tid,
    redirectUrl: json.data.redirectUrl || input.returnUrl,
    status: json.data.status,
  };
}
