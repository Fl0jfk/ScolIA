import "server-only";

import { NextResponse } from "next/server";
import { MessagingError } from "@/app/lib/messaging/service";

/** Réponse d’erreur uniforme des routes messagerie. */
export function messagingErrorResponse(
  error: unknown,
  tag: string,
  fallback: string,
): NextResponse {
  if (error instanceof MessagingError) {
    return NextResponse.json(
      { error: error.message, code: error.code },
      { status: error.status },
    );
  }
  console.error(`[messaging/${tag}]`, error);
  return NextResponse.json({ error: fallback }, { status: 500 });
}

export async function readJsonBody(req: Request): Promise<Record<string, unknown>> {
  try {
    const body = (await req.json()) as unknown;
    if (body && typeof body === "object" && !Array.isArray(body)) {
      return body as Record<string, unknown>;
    }
  } catch {
    /* corps invalide */
  }
  throw new MessagingError("Requête invalide.", 400, "INVALID_BODY");
}
