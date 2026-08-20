import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { uploadDocumentFile, type DocumentScope } from "@/app/lib/documents-cloud";

export const maxDuration = 60;

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const formData = await req.formData();
    const scope = (String(formData.get("scope") || "personal")) as DocumentScope;
    const shareIdRaw = formData.get("shareId");
    const shareId = shareIdRaw ? String(shareIdRaw) : null;
    const path = String(formData.get("path") ?? "");

    if (scope !== "personal" && scope !== "shared") {
      return NextResponse.json({ error: "Scope invalide." }, { status: 400 });
    }

    const files = formData.getAll("file").filter((f): f is File => f instanceof File);
    if (files.length === 0) {
      return NextResponse.json({ error: "Aucun fichier." }, { status: 400 });
    }

    const errors: string[] = [];
    for (const file of files) {
      const relPath = String(formData.get(`relPath:${file.name}`) ?? file.name).replace(/\\/g, "/");
      const parts = relPath.split("/").filter(Boolean);
      const fileName = parts.pop() ?? file.name;
      const subDir = parts.join("/");
      const base = path.endsWith("/") ? path : path ? `${path}/` : "";
      const targetPath = subDir ? `${base}${subDir}/` : base;

      const buffer = Buffer.from(await file.arrayBuffer());
      const result = await uploadDocumentFile(
        gate.ctx.userId,
        scope,
        shareId,
        targetPath,
        fileName,
        buffer,
        file.type,
      );
      if (!result.ok) {
        errors.push(`${file.name}: ${result.error}`);
        if (result.used !== undefined) {
          return NextResponse.json(
            { error: result.error, used: result.used, quota: result.quota },
            { status: 413 },
          );
        }
      }
    }

    if (errors.length > 0) {
      return NextResponse.json({ error: errors.join(" ") }, { status: 400 });
    }

    return NextResponse.json({ success: true, count: files.length });
  } catch (e) {
    console.error("[documents/upload]", e);
    return NextResponse.json({ error: "Échec de l'envoi." }, { status: 500 });
  }
}
