import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireModule } from "@/app/lib/intranet-auth";
import { fournituresPublicFileApiUrl } from "@/app/lib/fournitures-public-urls";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

const ALLOWED_KINDS = new Set(["ecole", "college", "lycee", "colbert", "arbs"]);

function normalizeUploadKind(kind: string): string | null {
  const k = kind.trim().toLowerCase();
  if (k === "colbert") return "ecole";
  if (k === "arbs") return "lycee";
  if (ALLOWED_KINDS.has(k)) return k;
  return null;
}

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "document.pdf";
}

/** Prépare un upload PDF partenaire (par cycle) vers S3 (servi via /api/fournitures/file). */
export async function POST(req: Request) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;

  try {
    const { fileName, fileType, kind } = await req.json();
    const docKind = normalizeUploadKind(String(kind || ""));
    if (!docKind) {
      return NextResponse.json(
        { error: "Type de document invalide (ecole, college ou lycee)." },
        { status: 400 },
      );
    }

    const type = String(fileType || "").trim().toLowerCase();
    if (type !== "application/pdf" && !String(fileName || "").toLowerCase().endsWith(".pdf")) {
      return NextResponse.json({ error: "Seuls les fichiers PDF sont acceptés." }, { status: 400 });
    }

    const fileKey = s3Key(
      `toolbox/fournitures/${docKind}/${Date.now()}-${safeFileName(String(fileName || `${docKind}.pdf`))}`,
    );
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      ContentType: "application/pdf",
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });

    return NextResponse.json({
      uploadUrl,
      fileUrl: fournituresPublicFileApiUrl(fileKey),
    });
  } catch (error) {
    console.error("[toolbox/fournitures/upload]", error);
    return NextResponse.json({ error: "Erreur lors de la préparation de l'upload." }, { status: 500 });
  }
}
