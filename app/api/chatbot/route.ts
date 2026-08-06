import { NextResponse } from "next/server";
import { isEleveBienEtreProfile, intranetRolesFromUnknown } from "@/app/lib/bien-etre-profile";
import { runBrainChat } from "@/app/lib/brain-ai/run-chat";
import type { BrainToolCtx } from "@/app/lib/brain-ai/types";
import { isOrgAdminFromPublicMetadata, safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { getMistralApiKey } from "@/app/lib/tenant-config";

export const runtime = "nodejs";

type ChatRequest = {
  message?: string;
  audience?: "public" | "private";
  history?: Array<{ role: "user" | "assistant"; content: string }>;
  conversationState?: unknown;
  confirm?: boolean;
  confirmAction?: { tool: string; args: Record<string, unknown> } | null;
  choiceApply?: {
    tool: string;
    field: string;
    value?: string;
    values?: string[];
    draftArgs: Record<string, unknown>;
  } | null;
  attachments?: Array<{ key: string; fileName: string; contentType?: string }>;
};

/** Soft rate-limit en mémoire (best-effort sur instance). */
const RATE = new Map<string, { count: number; resetAt: number }>();
const RATE_WINDOW_MS = 60_000;
const RATE_MAX = 30;

function rateLimit(key: string): boolean {
  const now = Date.now();
  const cur = RATE.get(key);
  if (!cur || cur.resetAt < now) {
    RATE.set(key, { count: 1, resetAt: now + RATE_WINDOW_MS });
    return true;
  }
  if (cur.count >= RATE_MAX) return false;
  cur.count += 1;
  return true;
}

export async function POST(req: Request) {
  try {
    const user = await safeCurrentUser();
    if (user && isEleveBienEtreProfile(intranetRolesFromUnknown(user.publicMetadata))) {
      return NextResponse.json(
        {
          error: "Utilise la bulle bien-être 💜 (bot d'écoute), pas l'assistant institutionnel.",
          code: "BIEN_ETRE_ONLY",
        },
        { status: 403 },
      );
    }

    const body = (await req.json()) as ChatRequest;
    const message = (body.message ?? "").trim();
    const confirm = Boolean(body.confirm);
    const choiceApply = body.choiceApply?.tool ? body.choiceApply : null;
    const audience = body.audience === "private" ? "private" : "public";
    const history = Array.isArray(body.history) ? body.history.slice(-12) : [];

    if (!message && !(confirm && body.confirmAction?.tool) && !choiceApply) {
      return NextResponse.json({ error: "message requis" }, { status: 400 });
    }

    const rateKey = user?.id || req.headers.get("x-forwarded-for") || "anon";
    if (!rateLimit(rateKey)) {
      return NextResponse.json(
        { error: "Trop de messages. Réessayez dans une minute.", code: "RATE_LIMIT" },
        { status: 429 },
      );
    }

    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const toolCtx: BrainToolCtx = {
      userId: user?.id ?? null,
      roles,
      isOrgAdmin: isOrgAdminFromPublicMetadata(user?.publicMetadata),
      audience: user && audience === "private" ? "private" : "public",
      confirmed: confirm,
      firstName: user?.firstName || undefined,
      lastName: user?.lastName || undefined,
      email: user?.primaryEmailAddress?.emailAddress || undefined,
    };

    const mistralKey = (await getMistralApiKey()) ?? null;
    const result = await runBrainChat({
      message: message || (confirm ? "(confirmation)" : choiceApply ? "(choix)" : "(confirmation)"),
      audience: toolCtx.audience,
      history,
      apiKey: mistralKey,
      toolCtx,
      conversationState: body.conversationState,
      confirm,
      confirmAction: body.confirmAction ?? null,
      choiceApply,
      attachments: Array.isArray(body.attachments) ? body.attachments : undefined,
    });

    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
