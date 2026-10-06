import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireModule } from "@/app/lib/intranet-auth";
import { loadAppConfig, saveTravelsModule } from "@/app/lib/app-config";
import {
  resolveTravelsAssistanceCardBytes,
  sanitizeAssistanceCardFileName,
  travelsAssistanceCardDownloadApiPath,
  travelsAssistanceCardObjectKey,
} from "@/app/lib/travels-assistance-card";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";

const PDF = "application/pdf";

/** État ou aperçu (`?raw=1`) de la carte d’assistance voyages. */
export async function GET(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const wantRaw = url.searchParams.get("raw") === "1";

  if (wantRaw) {
    const resolved = await resolveTravelsAssistanceCardBytes();
    if (!resolved) {
      return NextResponse.json({ error: "Carte d’assistance introuvable." }, { status: 404 });
    }
    return new NextResponse(Buffer.from(resolved.bytes), {
      status: 200,
      headers: {
        "Content-Type": PDF,
        "Content-Disposition": `inline; filename="${encodeURIComponent(resolved.fileName)}"`,
        "Cache-Control": "private, max-age=120",
      },
    });
  }

  const bundle = await loadAppConfig();
  const configured = Boolean(bundle.travels.assistanceCardS3Key?.trim());
  const resolved = configured ? await resolveTravelsAssistanceCardBytes() : null;
  return NextResponse.json({
    configured: Boolean(resolved),
    fileName: resolved?.fileName ?? bundle.travels.assistanceCardFileName ?? null,
    downloadUrl: resolved ? travelsAssistanceCardDownloadApiPath() : null,
  });
}

export async function POST(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const fileType = String(body.fileType || "").trim().toLowerCase();
    const fileName = sanitizeAssistanceCardFileName(String(body.fileName || "carte-assistance.pdf"));
    if (fileType !== PDF) {
      return NextResponse.json({ error: "Format attendu : PDF." }, { status: 400 });
    }

    const fileKey = travelsAssistanceCardObjectKey();
    const s3 = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const uploadUrl = await getSignedUrl(
      s3,
      new PutObjectCommand({ Bucket: bucket, Key: fileKey, ContentType: PDF }),
      { expiresIn: 3600 },
    );

    const bundle = await loadAppConfig();
    await saveTravelsModule({
      ...bundle.travels,
      assistanceCardS3Key: fileKey,
      assistanceCardFileName: fileName,
    });

    return NextResponse.json({
      uploadUrl,
      fileKey,
      fileName,
      downloadUrl: travelsAssistanceCardDownloadApiPath(),
    });
  } catch (error) {
    console.error("[settings/upload-assistance-card]", error);
    return NextResponse.json({ error: "Préparation upload impossible." }, { status: 500 });
  }
}

export async function DELETE() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  try {
    const bundle = await loadAppConfig();
    const { assistanceCardS3Key: _k, assistanceCardFileName: _n, ...rest } = bundle.travels;
    await saveTravelsModule(rest);
    return NextResponse.json({ ok: true });
  } catch (error) {
    console.error("[settings/upload-assistance-card DELETE]", error);
    return NextResponse.json({ error: "Suppression impossible." }, { status: 500 });
  }
}
