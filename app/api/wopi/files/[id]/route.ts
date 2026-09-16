import { NextRequest, NextResponse } from "next/server";
import {
  verifyWopiToken,
  getWopiLock,
  setWopiLock,
  clearWopiLock,
  headOfficeObjectMeta,
  wopiTimestamp,
} from "@/app/lib/office-wopi";

function extractToken(req: NextRequest): string | null {
  const q = req.nextUrl.searchParams.get("access_token");
  if (q) return q;
  const auth = req.headers.get("authorization");
  if (auth?.toLowerCase().startsWith("bearer ")) return auth.slice(7).trim();
  return null;
}

function wopiError(status: number, message: string) {
  return new NextResponse(message, { status });
}

type Ctx = { params: Promise<{ id: string }> };

type Claims = NonNullable<ReturnType<typeof verifyWopiToken>>;

function parseClaims(req: NextRequest, fileId: string): Claims | NextResponse {
  const token = extractToken(req);
  if (!token) return wopiError(401, "Missing access_token");
  const claims = verifyWopiToken(token);
  if (!claims || claims.fileId !== fileId) return wopiError(401, "Invalid access_token");
  return claims;
}

/** CheckFileInfo + Lock/Unlock/RefreshLock */
export async function GET(req: NextRequest, ctx: Ctx) {
  const { id: fileId } = await ctx.params;
  const claims = parseClaims(req, fileId);
  if (claims instanceof NextResponse) return claims;

  const meta = await headOfficeObjectMeta(claims.storageKey, claims.dataBucket);
  const lastModified = wopiTimestamp(meta.lastModified);
  return NextResponse.json({
    BaseFileName: claims.fileName,
    Size: meta.size,
    OwnerId: claims.ownerUserId,
    UserId: claims.userId,
    UserFriendlyName: claims.userDisplayName,
    Version: `${meta.size}-${lastModified}`,
    UserCanWrite: claims.canWrite,
    UserCanNotWriteRelative: true,
    SupportsLocks: true,
    SupportsUpdate: true,
    SupportsGetLock: true,
    SupportsExtendedLockLength: true,
    LastModifiedTime: lastModified,
  });
}

export async function POST(req: NextRequest, ctx: Ctx) {
  const { id: fileId } = await ctx.params;
  const claims = parseClaims(req, fileId);
  if (claims instanceof NextResponse) return claims;

  const override = req.headers.get("x-wopi-override")?.toUpperCase() || "";
  const lockHeader = req.headers.get("x-wopi-lock") || "";

  if (!claims.canWrite && ["LOCK", "UNLOCK", "REFRESH_LOCK"].includes(override)) {
    return wopiError(404, "Read-only");
  }

  if (override === "LOCK" || override === "REFRESH_LOCK") {
    const current = await getWopiLock(fileId);
    if (current && current !== lockHeader) {
      const res = wopiError(409, "Lock mismatch");
      res.headers.set("X-WOPI-Lock", current);
      return res;
    }
    await setWopiLock(fileId, lockHeader || crypto.randomUUID());
    const res = new NextResponse(null, { status: 200 });
    res.headers.set("X-WOPI-Lock", lockHeader);
    return res;
  }

  if (override === "UNLOCK") {
    const current = await getWopiLock(fileId);
    if (current && lockHeader && current !== lockHeader) {
      const res = wopiError(409, "Lock mismatch");
      res.headers.set("X-WOPI-Lock", current);
      return res;
    }
    await clearWopiLock(fileId);
    return new NextResponse(null, { status: 200 });
  }

  if (override === "GET_LOCK") {
    const current = (await getWopiLock(fileId)) || "";
    const res = new NextResponse(null, { status: 200 });
    res.headers.set("X-WOPI-Lock", current);
    return res;
  }

  return wopiError(501, `Override ${override || "(none)"} not implemented`);
}

export async function OPTIONS() {
  return new NextResponse(null, {
    status: 204,
    headers: {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers":
        "Authorization, Content-Type, X-WOPI-Override, X-WOPI-Lock, X-WOPI-OldLock",
    },
  });
}
