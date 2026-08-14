import "server-only";

import { GetObjectCommand, HeadObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { NextResponse } from "next/server";
import { isAllowedRentreeS3Key } from "@/app/lib/rentree-public-urls";
import { getBucketName, getObjectBytes, getS3Client } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { candidateTravelsS3Keys, resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";

export function rentreeKeyFromApiHref(href: string): string | null {
  const trimmed = href.trim();
  if (!trimmed) return null;
  try {
    const base = trimmed.startsWith("/") ? "https://local.invalid" : undefined;
    const u = new URL(trimmed, base);
    if (!u.pathname.endsWith("/api/rentree/file")) return null;
    const key = u.searchParams.get("key")?.trim() || "";
    return key && isAllowedRentreeS3Key(key) ? key : null;
  } catch {
    return null;
  }
}

function contentTypeForKey(key: string): string {
  const lower = key.toLowerCase();
  if (lower.endsWith(".pdf")) return "application/pdf";
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  if (lower.endsWith(".webp")) return "image/webp";
  if (lower.endsWith(".docx")) {
    return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
  }
  if (lower.endsWith(".xlsx")) {
    return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
  }
  return "application/octet-stream";
}

function fileNameFromRentreeKey(key: string): string {
  const base = key.split("/").pop() || "document";
  try {
    return decodeURIComponent(base);
  } catch {
    return base;
  }
}

async function rentreeKeyCandidates(key: string): Promise<string[]> {
  const fromTravels = await candidateTravelsS3Keys(key, key);
  const out = [...fromTravels];
  const add = (k: string) => {
    const n = k.replace(/^\/+/, "");
    if (n && !out.includes(n)) out.push(n);
  };
  if (key.startsWith("toolbox/rentree/")) {
    add(key.replace("toolbox/rentree/", "documents/rentree/"));
  }
  if (key.startsWith("documents/rentree/")) {
    add(key.replace("documents/rentree/", "toolbox/rentree/"));
  }
  const resolved = await resolveTravelsS3ObjectKey(key, key);
  if (resolved) add(resolved);
  return out;
}

async function loadRentreeFileBytes(
  key: string,
): Promise<{ bytes: Buffer; resolvedKey: string } | null> {
  if (!key || !isAllowedRentreeS3Key(key)) return null;
  for (const candidate of await rentreeKeyCandidates(key)) {
    const bytes = await getObjectBytes(candidate);
    if (bytes?.length) return { bytes, resolvedKey: candidate };
  }
  return null;
}

export function rentreeFileNotFoundHtml(key: string): string {
  const name = fileNameFromRentreeKey(key);
  return `<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Document introuvable</title>
  <style>
    body { font-family: system-ui, sans-serif; margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f8fafc; color: #0f172a; }
    main { max-width: 32rem; padding: 2rem; background: #fff; border: 1px solid #e2e8f0; border-radius: 1rem; box-shadow: 0 10px 30px rgba(15,23,42,.06); }
    h1 { font-size: 1.25rem; margin: 0 0 .75rem; }
    p { margin: 0 0 .5rem; line-height: 1.5; color: #475569; font-size: .95rem; }
    code { font-size: .8rem; word-break: break-all; }
    a { color: #4f46e5; font-weight: 700; }
  </style>
</head>
<body>
  <main>
    <h1>Document introuvable</h1>
    <p>Le fichier <strong>${name.replace(/</g, "&lt;")}</strong> n&apos;est pas disponible pour le moment.</p>
    <p>Vérifiez qu&apos;il a bien été enregistré dans la boîte à outils (Rentrée), ou contactez l&apos;établissement.</p>
    <p><a href="/rentree">← Retour à la rentrée</a></p>
    <p style="margin-top:1rem"><code>${key.replace(/</g, "&lt;")}</code></p>
  </main>
</body>
</html>`;
}

async function getSignedRentreeReadUrl(
  resolvedKey: string,
  opts?: { download?: boolean },
): Promise<string | null> {
  const key = s3Key(resolvedKey);
  const client = await getS3Client();
  const bucket = await getBucketName();
  const fileName = fileNameFromRentreeKey(key).replace(/"/g, "");
  const disposition = opts?.download ? "attachment" : "inline";

  try {
    await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
    const command = new GetObjectCommand({
      Bucket: bucket,
      Key: key,
      ResponseContentType: contentTypeForKey(key),
      ResponseContentDisposition: `${disposition}; filename="${fileName}"`,
    });
    return await getSignedUrl(client, command, { expiresIn: 3600 });
  } catch {
    return null;
  }
}

/** Redirige vers S3 (fiable en prod) ou renvoie le fichier en secours. */
export async function createRentreeFileServeResponse(
  key: string,
  opts?: { download?: boolean },
): Promise<Response> {
  if (!key || !isAllowedRentreeS3Key(key)) {
    return new Response(rentreeFileNotFoundHtml(key || "inconnu"), {
      status: 400,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  const resolvedKey = (await resolveTravelsS3ObjectKey(key, key)) || key;
  const signed = await getSignedRentreeReadUrl(resolvedKey, opts);
  if (signed) {
    return Response.redirect(signed, 307);
  }

  const loaded = await loadRentreeFileBytes(key);
  if (!loaded) {
    return new Response(rentreeFileNotFoundHtml(key), {
      status: 404,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  return buildRentreeFileResponse(loaded.bytes, loaded.resolvedKey, opts);
}

function buildRentreeFileResponse(
  bytes: Buffer,
  resolvedKey: string,
  opts?: { download?: boolean },
): NextResponse {
  const fileName = fileNameFromRentreeKey(resolvedKey);
  const safeName = fileName.replace(/"/g, "");
  const disposition = opts?.download ? "attachment" : "inline";
  const isPdf = resolvedKey.toLowerCase().endsWith(".pdf");
  const body = new Uint8Array(bytes);

  return new NextResponse(body, {
    headers: {
      "Content-Type": contentTypeForKey(resolvedKey),
      "Content-Disposition": `${disposition}; filename="${safeName}"`,
      "Content-Length": String(body.byteLength),
      "Cache-Control": "private, max-age=300",
      ...(isPdf ? { "X-Content-Type-Options": "nosniff" } : {}),
    },
  });
}
