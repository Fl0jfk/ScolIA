import { resolveSession } from "@/app/lib/intranet-session";
import { NextRequest } from "next/server";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { GetObjectCommand } from "@aws-sdk/client-s3";

import { getClerkClientForTenant } from "@/app/lib/tenant-clerk";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  getBucketName,
  getJson,
  getS3Client,
  getSignedReadUrl,
  putJson,
} from "@/app/lib/s3-storage";
import { getTenantAwsRegion } from "@/app/lib/tenant-config";
import { isSafeS3RelativeKey } from "@/app/lib/s3-path";
const FILE_KEY = "chat/messages.json";

async function readMessagesFromS3( shouldSign: boolean = true) {
  try {
    const hit = await getJson<unknown[]>( FILE_KEY);
    const messages = Array.isArray(hit?.data) ? hit.data : [];
    if (!shouldSign) return messages;

    const bucket = await getBucketName();
    const client = await getS3Client();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const signedMessages = await Promise.all(
      messages.map(async (m: any) => {
        if (!m.content?.includes?.("[IMG]") && !m.content?.includes?.("[DOC]")) return m;
        try {
          const isImg = m.content.startsWith("[IMG]");
          let fileUrl = "";
          let fileName = "";
          if (isImg) {
            fileUrl = m.content.replace("[IMG]", "");
          } else {
            const parts = m.content.split("|");
            fileName = parts[0];
            fileUrl = parts[1];
          }
          const region = await getTenantAwsRegion();
          const marker = `${bucket}.s3.${region}.amazonaws.com/`;
          const idx = fileUrl.indexOf(marker);
          const fileKey = idx >= 0 ? fileUrl.slice(idx + marker.length) : "";
          if (fileKey && isSafeS3RelativeKey(fileKey)) {
            const signedUrl =
              (await getSignedReadUrl(fileKey, 3600)) ||
              (await getSignedUrl(client, new GetObjectCommand({ Bucket: bucket, Key: fileKey }), { expiresIn: 3600 }));
            m.content = isImg ? `[IMG]${signedUrl}` : `${fileName}|${signedUrl}`;
          }
        } catch (err) {
          console.error("Erreur de signature individuelle:", err);
        }
        return m;
      }),
    );
    return signedMessages;
  } catch (e) {
    console.error("Erreur lecture messages S3:", e);
    return [];
  }
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const messages = await readMessagesFromS3( true);
  return new Response(JSON.stringify(messages), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const session = await resolveSession();
  const authUserId = session?.userId;
  if (!authUserId) return new Response("Non autorisé", { status: 401 });
  try {
    const client = await getClerkClientForTenant();
    const user = await client.users.getUser(userId);
    const body = await req.json();
    const messages = await readMessagesFromS3( false);
    const newMessage = {
      id: Date.now(),
      channel: body.channel,
      content: body.content,
      createdAt: new Date().toISOString(),
      authorName: body.isAnonymous ? "Anonyme" : `${user.firstName} ${user.lastName}`,
      authorId: body.isAnonymous ? null : userId,
      avatar: body.isAnonymous ? null : user.imageUrl,
    };
    messages.push(newMessage);
    await putJson(FILE_KEY, messages);
    return new Response(JSON.stringify(newMessage), { status: 200 });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } catch (error: any) {
    return new Response(JSON.stringify({ error: error.message }), { status: 500 });
  }
}

export async function DELETE(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
    const session = await resolveSession();
    const userId = session?.userId;
  if (!userId) return new Response("Non autorisé", { status: 401 });
  const { searchParams } = new URL(req.url);
  const messageId = searchParams.get("id");
  if (!messageId) return new Response("ID manquant", { status: 400 });
  try {
    const messages = await readMessagesFromS3( false);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const filteredMessages = messages.filter((m: any) => m.id.toString() !== messageId);
    await putJson(FILE_KEY, filteredMessages);
    return new Response(JSON.stringify({ success: true }), { status: 200 });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } catch (error: any) {
    return new Response(JSON.stringify({ error: error.message }), { status: 500 });
  }
}
