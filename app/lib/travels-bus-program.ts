import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";

export async function loadBusProgramAttachments(
  data: Record<string, unknown>,
): Promise<Array<{ filename: string; content: Buffer; contentType: string }>> {
  const attachments: Array<{ filename: string; content: Buffer; contentType: string }> = [];
  const transportRequest = data.transportRequest as {
    busProgramFile?: { url?: string; name?: string; s3Key?: string };
  } | undefined;
  const busFile = transportRequest?.busProgramFile;
  if (!busFile?.url) return attachments;

  try {
    const fileKey =
      busFile.s3Key ||
      (await parseTravelsS3KeyFromUrl(busFile.url)) ||
      null;
    if (!fileKey) return attachments;

    const s3Client = await getTenantDataS3Client();
    const command = new GetObjectCommand({ Bucket: await getTenantBucketName(), Key: fileKey });
    const presignedUrl = await getSignedUrl(s3Client, command, { expiresIn: 120 });
    const fileRes = await fetch(presignedUrl);
    if (fileRes.ok) {
      const arrayBuffer = await fileRes.arrayBuffer();
      attachments.push({
        filename: busFile.name || "Programme_de_route.pdf",
        content: Buffer.from(arrayBuffer),
        contentType: "application/pdf",
      });
    }
  } catch (e) {
    console.error("[travels-bus-program]", e);
  }
  return attachments;
}
