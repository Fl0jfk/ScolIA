import { NextRequest, NextResponse } from "next/server";
import {
  deletePendingRentreeSubmission,
  deliverConfirmedRentreeSubmission,
  loadPendingRentreeFileBytes,
  loadPendingRentreeSubmission,
  rentreeSubmissionResultUrl,
} from "@/app/lib/rentree-submissions";

export const runtime = "nodejs";
export const maxDuration = 60;

async function redirectToResult(query: Record<string, string>) {
  const url = await rentreeSubmissionResultUrl(query);
  return NextResponse.redirect(url);
}

export async function GET(req: NextRequest) {
  const token = req.nextUrl.searchParams.get("token")?.trim() ?? "";
  if (!token) return redirectToResult({ erreur: "lien_invalide" });

  try {
    const meta = await loadPendingRentreeSubmission(token);
    if (!meta) return redirectToResult({ erreur: "lien_invalide" });

    if (new Date(meta.expiresAt).getTime() < Date.now()) {
      await deletePendingRentreeSubmission(token);
      return redirectToResult({ erreur: "lien_expire" });
    }

    const bytes = await loadPendingRentreeFileBytes(meta.fileKey);
    if (!bytes) {
      await deletePendingRentreeSubmission(token);
      return redirectToResult({ erreur: "fichier_manquant" });
    }

    await deliverConfirmedRentreeSubmission(meta, bytes);
    await deletePendingRentreeSubmission(token);
    return redirectToResult({ ok: "1" });
  } catch (e) {
    console.error("[rentree/submissions] confirm:", e);
    return redirectToResult({ erreur: "serveur" });
  }
}
