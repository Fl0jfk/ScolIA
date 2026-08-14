import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { putObject } from "@/app/lib/s3-storage";
import { s3Key, sanitizeS3FileName } from "@/app/lib/s3-path";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";

export const maxDuration = 60;

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
    try {
    const formData = await req.formData();
    const file = formData.get("file") as File;
    if (!file) {
      return NextResponse.json({ error: "Aucun fichier trouvé" }, { status: 400 });
    }
    const buffer = Buffer.from(await file.arrayBuffer());
    const fileName = `${Date.now()}-${sanitizeS3FileName(file.name).replace(/\s+/g, "_")}`;
    const rel = `uploads/${fileName}`;
    await putObject( rel, buffer, file.type);
    const fileKey = s3Key( rel);
    const url = await publicS3UrlForKey(fileKey);
    return NextResponse.json({
      url,
      name: file.name,
      type: file.type,
      key: fileKey,
    });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } catch (error: any) {
    console.error("Erreur Upload S3:", error);
    return NextResponse.json({ error: error.message }, { status: 500 });
  }
}
