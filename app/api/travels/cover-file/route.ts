import { GetObjectCommand } from "@aws-sdk/client-s3";
import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  scoliaImageCoverBucket,
  scoliaImageCoverCdnClient,
} from "@/app/lib/travels-cover-cdn";
import { travelAutoCoverObjectKey } from "@/app/lib/travels-image-url";

export const runtime = "nodejs";

/**
 * Sert les covers auto (travels/auto/*) via l’app.
 * Contourne le 403 CDN quand l’objet S3 a été uploadé sans ACL public-read.
 */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const rawKey = String(new URL(req.url).searchParams.get("key") || "").trim();
    const key = travelAutoCoverObjectKey(rawKey);
    if (!key) {
      return NextResponse.json({ error: "Clé invalide" }, { status: 400 });
    }

    const client = scoliaImageCoverCdnClient();
    const obj = await client.send(
      new GetObjectCommand({
        Bucket: scoliaImageCoverBucket(),
        Key: key,
      }),
    );
    if (!obj.Body) {
      return NextResponse.json({ error: "Introuvable" }, { status: 404 });
    }

    const bytes = Buffer.from(await obj.Body.transformToByteArray());
    const contentType = obj.ContentType || "image/jpeg";
    return new NextResponse(bytes, {
      status: 200,
      headers: {
        "Content-Type": contentType,
        "Cache-Control": "private, max-age=86400",
        "Content-Length": String(bytes.length),
      },
    });
  } catch (err) {
    console.error("[travels/cover-file]", err);
    return NextResponse.json({ error: "Lecture impossible" }, { status: 502 });
  }
}
