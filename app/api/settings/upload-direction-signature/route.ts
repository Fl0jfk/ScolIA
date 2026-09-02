import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireModule } from "@/app/lib/intranet-auth";
import { loadAppConfig, saveEstablishments } from "@/app/lib/app-config";
import {
  directionSignatureObjectKey,
  directionSignaturePreviewApiPath,
  resolveDirectionSignatureBytes,
  sniffDirectionSignatureContentType,
} from "@/app/lib/direction-signature";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";

const ALLOWED = new Set(["image/png", "image/jpeg", "image/webp"]);

function extForType(type: string): "png" | "jpg" | "webp" {
  if (type === "image/jpeg") return "jpg";
  if (type === "image/webp") return "webp";
  return "png";
}

/**
 * Aperçu :
 * - `?establishmentId=ecole&raw=1` → image binaire (same-origin, pour <img>)
 * - `?establishmentId=ecole` → JSON `{ previewUrl }` (chemin API, pas d’URL S3)
 */
export async function GET(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const establishmentId = url.searchParams.get("establishmentId")?.trim() || "";
  if (!establishmentId) {
    return NextResponse.json({ error: "establishmentId requis." }, { status: 400 });
  }

  const wantRaw =
    url.searchParams.get("raw") === "1" || url.searchParams.get("format") === "image";

  const bytes = await resolveDirectionSignatureBytes(establishmentId);
  if (!bytes?.length) {
    if (wantRaw) {
      return NextResponse.json({ error: "Signature introuvable." }, { status: 404 });
    }
    return NextResponse.json({ establishmentId, previewUrl: null });
  }

  if (wantRaw) {
    const contentType = sniffDirectionSignatureContentType(bytes);
    return new NextResponse(Buffer.from(bytes), {
      status: 200,
      headers: {
        "Content-Type": contentType,
        "Cache-Control": "private, max-age=120",
      },
    });
  }

  return NextResponse.json({
    establishmentId,
    previewUrl: directionSignaturePreviewApiPath(establishmentId),
  });
}

export async function POST(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const establishmentId = String(body.establishmentId || "").trim();
    const fileType = String(body.fileType || "").trim().toLowerCase();
    if (!establishmentId) {
      return NextResponse.json({ error: "establishmentId requis." }, { status: 400 });
    }
    if (!ALLOWED.has(fileType)) {
      return NextResponse.json({ error: "Format : PNG, JPEG ou WebP." }, { status: 400 });
    }

    const bundle = await loadAppConfig();
    const est = bundle.establishments.find((e) => e.id === establishmentId);
    if (!est) {
      return NextResponse.json(
        { error: "Établissement introuvable. Enregistrez-le d’abord." },
        { status: 404 },
      );
    }

    const fileKey = directionSignatureObjectKey(establishmentId, extForType(fileType));
    const s3 = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const uploadUrl = await getSignedUrl(
      s3,
      new PutObjectCommand({ Bucket: bucket, Key: fileKey, ContentType: fileType }),
      { expiresIn: 3600 },
    );

    const next = bundle.establishments.map((e) =>
      e.id === establishmentId ? { ...e, signatureS3Key: fileKey } : e,
    );
    await saveEstablishments(next);

    // Pas d’URL S3 avant upload : le client affichera l’aperçu API après le PUT.
    return NextResponse.json({
      uploadUrl,
      fileKey,
      previewUrl: directionSignaturePreviewApiPath(establishmentId, Date.now()),
    });
  } catch (error) {
    console.error("[settings/upload-direction-signature]", error);
    return NextResponse.json({ error: "Préparation upload impossible." }, { status: 500 });
  }
}

export async function DELETE(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const establishmentId = String(body.establishmentId || "").trim();
    if (!establishmentId) {
      return NextResponse.json({ error: "establishmentId requis." }, { status: 400 });
    }
    const bundle = await loadAppConfig();
    const next = bundle.establishments.map((e) => {
      if (e.id !== establishmentId) return e;
      const { signatureS3Key: _removed, ...rest } = e;
      return rest;
    });
    await saveEstablishments(next);
    return NextResponse.json({ ok: true });
  } catch (error) {
    console.error("[settings/upload-direction-signature DELETE]", error);
    return NextResponse.json({ error: "Suppression impossible." }, { status: 500 });
  }
}
