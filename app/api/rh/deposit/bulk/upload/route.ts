import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import { uploadRhBulkToTemp } from "@/app/lib/rh/rh-ocr-batch";

export const maxDuration = 60;


/** Upload RH en vrac → S3 + OneDrive Temp (sans MSAL client). */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH." }, { status: 403 });
  }

  try {
    const form = await req.formData();
    const files = form.getAll("files").filter((f): f is File => f instanceof File);
    const single = form.get("file");
    if (single instanceof File) files.push(single);
    if (files.length === 0) {
      return NextResponse.json({ error: "Aucun fichier." }, { status: 400 });
    }

    const uploaded: Array<{ fileName: string; s3Key: string; tempPath: string }> = [];
    for (const file of files) {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const hit = await uploadRhBulkToTemp({
        fileName: file.name,
        bytes,
        contentType: file.type || "application/pdf",
      });
      uploaded.push({ fileName: file.name, ...hit });
    }

    return NextResponse.json({ items: uploaded });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Upload impossible." },
      { status: 500 },
    );
  }
}
