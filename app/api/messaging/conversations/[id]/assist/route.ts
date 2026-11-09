import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { AssistError, runMessagingAssist } from "@/app/lib/messaging/assist";
import { createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

export const maxDuration = 60;

type Ctx = { params: Promise<{ id: string }> };

const limiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
  name: "messaging-assist",
});

const MODES = ["reply", "rewrite", "summarize"] as const;
const TONES = ["neutre", "chaleureux", "formel", "court"] as const;

function err(error: string, code: string, status: number) {
  return NextResponse.json({ error, code }, { status });
}

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  const body = (await req.json().catch(() => null)) as Record<string, unknown> | null;
  if (!body || typeof body !== "object") return err("Requête invalide.", "BAD_REQUEST", 400);
  const mode = body.mode;
  if (typeof mode !== "string" || !(MODES as readonly string[]).includes(mode)) {
    return err("Mode invalide.", "INVALID_MODE", 400);
  }
  const instruction = typeof body.instruction === "string" ? body.instruction : undefined;
  const draft = typeof body.draft === "string" ? body.draft : undefined;
  if ((instruction?.length ?? 0) > 500) {
    return err("Consigne trop longue (500 caractères max).", "INSTRUCTION_TOO_LONG", 400);
  }
  if ((draft?.length ?? 0) > 4000) {
    return err("Brouillon trop long (4000 caractères max).", "DRAFT_TOO_LONG", 400);
  }
  if (mode === "rewrite" && !draft?.trim()) {
    return err("Écrivez d'abord un brouillon à améliorer.", "DRAFT_REQUIRED", 400);
  }
  const tone =
    typeof body.tone === "string" && (TONES as readonly string[]).includes(body.tone)
      ? (body.tone as (typeof TONES)[number])
      : undefined;

  const apiKey = process.env.MISTRAL_API_KEY?.trim();
  if (!apiKey) return err("Assistant indisponible.", "ASSIST_UNAVAILABLE", 503);

  if (!(await limiter.allow(gate.ctx.userId))) {
    return err("Trop de demandes. Réessayez dans quelques minutes.", "RATE_LIMITED", 429);
  }

  try {
    const { id } = await ctx.params;
    const text = await runMessagingAssist({
      etablissementId: gate.ctx.etablissementId,
      conversationId: id,
      userId: gate.ctx.userId,
      mode: mode as (typeof MODES)[number],
      instruction,
      draft,
      tone,
      apiKey,
    });
    return NextResponse.json({ text });
  } catch (error) {
    if (error instanceof AssistError) return err(error.message, error.code, error.status);
    console.error("[messaging/assist]", (error as Error)?.name);
    return err("Impossible de rédiger pour le moment.", "ASSIST_FAILED", 500);
  }
}
