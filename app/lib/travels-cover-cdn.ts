import "server-only";
import { GetObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { SCOLA_IMAGE_BUCKET } from "@/app/lib/scola-image";
import { travelAutoCoverObjectKey } from "@/app/lib/travels-image-url";

export function scoliaImageCoverCdnClient(): S3Client {
  const accessKeyId = process.env.ACCESS_KEY_ID?.trim();
  const secretAccessKey = process.env.SECRET_ACCESS_KEY?.trim();
  if (!accessKeyId || !secretAccessKey) {
    throw new Error("ACCESS_KEY_ID / SECRET_ACCESS_KEY manquants");
  }
  return new S3Client({
    region: "fr-par",
    endpoint: process.env.S3_ENDPOINT?.trim() || "https://s3.fr-par.scw.cloud",
    forcePathStyle: process.env.S3_FORCE_PATH_STYLE !== "false",
    credentials: { accessKeyId, secretAccessKey },
    requestChecksumCalculation: "WHEN_REQUIRED",
    responseChecksumValidation: "WHEN_REQUIRED",
  });
}

export function scoliaImageCoverBucket(): string {
  return process.env.IMAGE_BUCKET?.trim() || SCOLA_IMAGE_BUCKET;
}

/** Lit les octets d’une cover (S3 privé travels/auto ou URL HTTP publique). */
export async function loadTravelCoverImageBytes(
  urlOrKey: string | null | undefined,
): Promise<{ bytes: Buffer; contentType: string } | null> {
  const raw = String(urlOrKey || "").trim();
  if (!raw) return null;

  const key = travelAutoCoverObjectKey(raw);
  if (key) {
    try {
      const client = scoliaImageCoverCdnClient();
      const obj = await client.send(
        new GetObjectCommand({
          Bucket: scoliaImageCoverBucket(),
          Key: key,
        }),
      );
      if (!obj.Body) return null;
      const bytes = Buffer.from(await obj.Body.transformToByteArray());
      return {
        bytes,
        contentType: obj.ContentType || "image/jpeg",
      };
    } catch (err) {
      console.error("[travels-cover] S3 get failed", key, err);
      return null;
    }
  }

  try {
    const res = await fetch(raw, {
      headers: { Accept: "image/*,*/*" },
      redirect: "follow",
    });
    if (!res.ok) return null;
    const contentType = res.headers.get("content-type") || "image/jpeg";
    if (!contentType.startsWith("image/")) return null;
    const bytes = Buffer.from(await res.arrayBuffer());
    if (!bytes.length) return null;
    return { bytes, contentType };
  } catch (err) {
    console.error("[travels-cover] fetch failed", raw, err);
    return null;
  }
}
