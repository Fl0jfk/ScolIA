import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { getSignedReadUrl } from "@/app/lib/s3-storage";
import {
  assertCanReadFile,
  parseFileShareIdFromRel,
  resolveFileShareReadKey,
  storageKeyForItem,
  type DocumentScope,
} from "@/app/lib/documents-cloud";

export async function GET(req: NextRequest) {
  const gate = await requireModule("documents");
  if (!gate.ok) return gate.response;

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  const { searchParams } = new URL(req.url);
  const scope = (searchParams.get("scope") || "personal") as DocumentScope;
  const shareId = searchParams.get("shareId");
  const relPath = searchParams.get("path") || searchParams.get("key") || "";

  if (!relPath) {
    return NextResponse.json({ error: "Fichier absent." }, { status: 400 });
  }

  const fileShareId = parseFileShareIdFromRel(relPath);
  let storageKey: string;

  if (fileShareId) {
    const shared = await resolveFileShareReadKey(gate.ctx.userId, fileShareId);
    if (!shared.ok) return NextResponse.json({ error: shared.error }, { status: 403 });
    storageKey = shared.key;
  } else {
    const access = await assertCanReadFile(gate.ctx.userId, scope, shareId);
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: 403 });
    const resolved = storageKeyForItem(gate.ctx.userId, scope, shareId, relPath);
    if (!resolved.ok) return NextResponse.json({ error: resolved.error }, { status: 400 });
    storageKey = resolved.key;
  }

  try {
    const url = await getSignedReadUrl(storageKey, 3600);
    if (!url) return NextResponse.json({ error: "Fichier introuvable." }, { status: 404 });

    await writeDataAccessAudit({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      resourceType: "document",
      resourceId: relPath,
      action: "download",
      req,
      metadata: { scope, storageKey },
    });

    return NextResponse.json({ url });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
