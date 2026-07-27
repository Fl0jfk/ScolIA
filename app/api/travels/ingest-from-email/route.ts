import { NextResponse } from "next/server";
import {
  ingestSecretFromEnv,
  ingestTravelEmail,
  isAllowedIncomingKey,
} from "@/app/lib/travel-email-ingest";

export const maxDuration = 300;

export async function POST(req: Request) {
  const secret = ingestSecretFromEnv();
  if (!secret) {
    return NextResponse.json(
      {
        error:
          "Secret d'ingestion non configuré. Définir TRAVEL_EMAIL_INGEST_SECRET (ou INGEST_SECRET) sur le conteneur Scaleway.",
      },
      { status: 503 },
    );
  }
  const hdr = (req.headers.get("x-travel-email-ingest-secret") || "").trim();
  if (hdr !== secret) {
    return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
  }

  let body: {
    s3Key?: string;
    fromEmail?: string;
    subject?: string;
    snippet?: string;
    emailBodyPlain?: string;
    gmailMessageId?: string;
    originalFilename?: string;
  };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const { s3Key, fromEmail, subject, snippet, emailBodyPlain, gmailMessageId, originalFilename } = body;
  if (!fromEmail || !gmailMessageId) {
    return NextResponse.json({ error: "fromEmail et gmailMessageId requis" }, { status: 400 });
  }

  const hasPdf = Boolean(s3Key);
  const textContent = (emailBodyPlain || snippet || "").trim();
  if (!hasPdf && textContent.length < 8) {
    return NextResponse.json(
      { error: "s3Key ou emailBodyPlain/snippet requis pour l'analyse IA" },
      { status: 400 },
    );
  }
  if (s3Key && !isAllowedIncomingKey(s3Key)) {
    return NextResponse.json({ error: "Clé S3 non autorisée" }, { status: 400 });
  }

  const result = await ingestTravelEmail({
    s3Key,
    fromEmail,
    subject,
    snippet,
    emailBodyPlain,
    gmailMessageId,
    originalFilename,
  });

  if (result.pending) {
    return NextResponse.json(
      {
        ok: true,
        accepted: true,
        completed: false,
        pending: true,
        detail: result.detail,
      },
      { status: 202 },
    );
  }

  const status = result.ok === false && result.reason === "contenu_insuffisant" ? 400 : 200;
  return NextResponse.json(
    {
      ok: result.ok,
      completed: result.completed,
      failed: result.failed,
      matched: result.matched,
      duplicate: result.duplicate,
      tripId: result.tripId ?? null,
      reason: result.reason ?? null,
      devisId: result.devisId ?? null,
      messageId: result.messageId ?? null,
      safeToMarkRead: result.safeToMarkRead,
    },
    { status },
  );
}
