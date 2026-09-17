import { NextResponse } from "next/server";
import { processRdvInscriptionReconfirmMails } from "@/app/lib/rdv-inscription-service";

/**
 * Cron J-7 reconfirmation RDV inscription.
 * Auth : header `Authorization: Bearer <RDV_INSCRIPTION_CRON_SECRET>`
 * ou body `{ cronSecret }` (repli TRAVELS_CRON_SECRET).
 */
export async function POST(req: Request) {
  try {
    const secret =
      process.env.RDV_INSCRIPTION_CRON_SECRET?.trim() ||
      process.env.TRAVELS_CRON_SECRET?.trim() ||
      "";
    const auth = req.headers.get("authorization") || "";
    const bearer = auth.startsWith("Bearer ") ? auth.slice(7).trim() : "";
    let bodySecret = "";
    try {
      const body = (await req.json()) as { cronSecret?: string };
      bodySecret = String(body.cronSecret || "").trim();
    } catch {
      /* body optionnel */
    }

    if (!secret || (bearer !== secret && bodySecret !== secret)) {
      return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
    }

    const result = await processRdvInscriptionReconfirmMails();
    return NextResponse.json({
      success: true,
      sent: result.sent,
      errors: result.errors,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
