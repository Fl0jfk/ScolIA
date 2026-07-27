import { NextResponse } from "next/server";
import { ingestSecretFromEnv } from "@/app/lib/travel-email-ingest";
import { pollTravelImapInbox, travelImapConfigured } from "@/app/lib/travel-imap-poller";

export const maxDuration = 300;

/**
 * Cron Scaleway → poll IMAP (OVH / Infomaniak / …) de la boîte transport partagée.
 */
export async function POST(req: Request) {
  const secret = ingestSecretFromEnv();
  if (!secret) {
    return NextResponse.json(
      {
        error:
          "Secret d'ingestion non configuré. Définir TRAVEL_EMAIL_INGEST_SECRET sur le conteneur Scaleway.",
      },
      { status: 503 },
    );
  }
  const hdr = (req.headers.get("x-travel-email-ingest-secret") || "").trim();
  if (hdr !== secret) {
    return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
  }

  if (!travelImapConfigured()) {
    return NextResponse.json(
      {
        error:
          "IMAP non configuré. Définir MAILER_EMAIL, MAILER_PASS, MAILER_HOST (OVH).",
      },
      { status: 503 },
    );
  }

  try {
    const summary = await pollTravelImapInbox({ maxMessages: 25 });
    return NextResponse.json({ ok: true, ...summary });
  } catch (e) {
    console.error("[poll-email]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur polling IMAP" },
      { status: 500 },
    );
  }
}
