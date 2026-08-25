import { NextResponse } from "next/server";
import { getAbsenceDocumentKeys } from "@/app/lib/absences-documents";
import { canViewAbsenceAttachment } from "@/app/lib/absences-types";
import { getAbsenceOrLegacyRecord } from "@/app/lib/absences-legacy-convocations";
import { loadAppConfig } from "@/app/lib/app-config";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getSignedReadUrl } from "@/app/lib/s3-storage";
import { resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  const { searchParams } = new URL(req.url);
  const id = String(searchParams.get("id") || "").trim();
  const docIndexRaw = searchParams.get("index");
  const docIndex = docIndexRaw === null || docIndexRaw === "" ? 0 : Number(docIndexRaw);
  if (!id) return NextResponse.json({ error: "Paramètre 'id' manquant." }, { status: 400 });

  try {
    const record = await getAbsenceOrLegacyRecord(id);
    if (!record) return NextResponse.json({ error: "Absence introuvable" }, { status: 404 });

    const bundle = await loadAppConfig();
    const ctx = { establishments: bundle.establishments, userId };
    if (!canViewAbsenceAttachment(record, userId, roles, ctx)) {
      return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
    }

    const urls: string[] = [];

    const justUrl = record.justification?.fileUrl?.trim();
    if (justUrl) {
      const justKey = await resolveTravelsS3ObjectKey(justUrl);
      if (justKey) {
        const signed = await getSignedReadUrl(justKey, 60 * 10);
        if (signed) urls.push(signed);
      } else if (/^https?:\/\//i.test(justUrl)) {
        urls.push(justUrl);
      }
    }

    const keys = getAbsenceDocumentKeys(record);
    for (const key of keys) {
      const signed = await getSignedReadUrl(key, 60 * 10);
      if (signed) urls.push(signed);
    }

    if (urls.length === 0) {
      return NextResponse.json({ error: "Document introuvable." }, { status: 404 });
    }

    const index = Number.isFinite(docIndex)
      ? Math.max(0, Math.min(urls.length - 1, Math.floor(docIndex)))
      : 0;

    return NextResponse.json({ url: urls[index], urls, count: urls.length });
  } catch (error) {
    console.error("Absences document-url error:", error);
    return NextResponse.json({ error: "Erreur récupération document absence" }, { status: 500 });
  }
}
