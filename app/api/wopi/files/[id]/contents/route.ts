import { NextRequest, NextResponse } from "next/server";
import {
  getObjectBytes,
  getObjectBytesInBucket,
  putObject,
  putObjectInBucket,
} from "@/app/lib/s3-storage";
import {
  verifyWopiToken,
  getWopiLock,
  wopiMime,
  headOfficeObjectSize,
  type WopiAccessClaims,
} from "@/app/lib/office-wopi";
import { snapshotOfficeVersion } from "@/app/lib/office-versions";

function extractToken(req: NextRequest): string | null {
  const q = req.nextUrl.searchParams.get("access_token");
  if (q) return q;
  const auth = req.headers.get("authorization");
  if (auth?.toLowerCase().startsWith("bearer ")) return auth.slice(7).trim();
  return null;
}

type Ctx = { params: Promise<{ id: string }> };

async function readBytes(claims: WopiAccessClaims): Promise<Buffer | null> {
  if (claims.dataBucket?.trim()) {
    return getObjectBytesInBucket(claims.dataBucket.trim(), claims.storageKey);
  }
  return getObjectBytes(claims.storageKey);
}

async function writeBytes(claims: WopiAccessClaims, buffer: Buffer): Promise<void> {
  const mime = wopiMime(claims.fileName);
  if (claims.dataBucket?.trim()) {
    await putObjectInBucket(claims.dataBucket.trim(), claims.storageKey, buffer, mime);
    return;
  }
  await putObject(claims.storageKey, buffer, mime);
}

/** GetFile */
export async function GET(req: NextRequest, ctx: Ctx) {
  const { id: fileId } = await ctx.params;
  const token = extractToken(req);
  if (!token) return new NextResponse("Missing access_token", { status: 401 });
  const claims = verifyWopiToken(token);
  if (!claims || claims.fileId !== fileId) {
    return new NextResponse("Invalid access_token", { status: 401 });
  }

  const bytes = await readBytes(claims);
  if (!bytes) {
    // Guérison : brouillon créé mais objet S3 absent (mauvais bucket Host / upload raté).
    if (claims.isOwner && claims.canWrite && claims.kind) {
      try {
        const { buildBlankOfficeBuffer } = await import("@/app/lib/office-odf-blank");
        const blank = await buildBlankOfficeBuffer(claims.kind);
        await writeBytes(claims, blank);
        return new NextResponse(new Uint8Array(blank), {
          status: 200,
          headers: {
            "Content-Type": wopiMime(claims.fileName),
            "Content-Length": String(blank.length),
            "X-WOPI-ItemVersion": String(blank.length),
          },
        });
      } catch (e) {
        console.error("[wopi] blank heal failed", e);
      }
    }
    return new NextResponse("Not found", { status: 404 });
  }
  return new NextResponse(new Uint8Array(bytes), {
    status: 200,
    headers: {
      "Content-Type": wopiMime(claims.fileName),
      "Content-Length": String(bytes.length),
      "X-WOPI-ItemVersion": String(bytes.length),
    },
  });
}

/** PutFile */
export async function POST(req: NextRequest, ctx: Ctx) {
  const { id: fileId } = await ctx.params;
  const token = extractToken(req);
  if (!token) return new NextResponse("Missing access_token", { status: 401 });
  const claims = verifyWopiToken(token);
  if (!claims || claims.fileId !== fileId) {
    return new NextResponse("Invalid access_token", { status: 401 });
  }
  if (!claims.canWrite) return new NextResponse("Read-only", { status: 409 });

  const lockHeader = req.headers.get("x-wopi-lock") || "";
  const currentLock = await getWopiLock(fileId);
  const size = await headOfficeObjectSize(claims.storageKey, claims.dataBucket);
  if (size > 0 && currentLock && lockHeader && currentLock !== lockHeader) {
    const res = new NextResponse("Lock mismatch", { status: 409 });
    res.headers.set("X-WOPI-Lock", currentLock);
    return res;
  }

  const ab = await req.arrayBuffer();
  const buffer = Buffer.from(ab);
  if (buffer.length === 0) {
    return new NextResponse("Empty body", { status: 400 });
  }

  try {
    await snapshotOfficeVersion({
      ownerUserId: claims.ownerUserId,
      fileId,
      currentStorageKey: claims.storageKey,
      dataBucket: claims.dataBucket,
    });
  } catch (e) {
    console.error("[wopi] version snapshot", e);
  }

  await writeBytes(claims, buffer);

  return NextResponse.json({
    Name: claims.fileName,
    Size: buffer.length,
    LastModifiedTime: new Date().toISOString(),
  });
}
