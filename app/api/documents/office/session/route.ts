import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  makeOfficeFileId,
  resolveOfficeStorage,
  claimEditorSession,
  getEditorSession,
} from "@/app/lib/office-wopi";
import { listOfficeVersions, restoreOfficeVersion } from "@/app/lib/office-versions";
import { requireTenantId } from "@/app/lib/tenant-scope";
import type { OfficeScope } from "@/app/lib/office-types";
import { OFFICE_KIND_META } from "@/app/lib/office-types";

function parseScope(raw: string | null): OfficeScope | null {
  const s = String(raw || "");
  return s === "personal" || s === "shared" || s === "fileshare" ? s : null;
}

export async function GET(req: NextRequest) {
  const gate = await requireModule("documents");
  if (!gate.ok) return gate.response;
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  const { searchParams } = new URL(req.url);
  const scope = parseScope(searchParams.get("scope"));
  const relPath = searchParams.get("path") || "";
  const shareId = searchParams.get("shareId");
  const fileShareId = searchParams.get("fileShareId");
  const sessionId = searchParams.get("sessionId") || "";
  const takeover = searchParams.get("takeover") === "1";

  if (!scope || !relPath) {
    return NextResponse.json({ error: "Paramètres manquants." }, { status: 400 });
  }

  const resolved = await resolveOfficeStorage({
    userId: gate.ctx.userId,
    scope,
    shareId,
    fileShareId,
    relPath,
  });
  if (!resolved.ok) return NextResponse.json({ error: resolved.error }, { status: 403 });

  const fileId = makeOfficeFileId({
    etablissementId: tenant.ctx.etablissementId,
    scope,
    shareId,
    fileShareId,
    relPath,
  });

  if (sessionId) {
    const claim = await claimEditorSession({
      fileId,
      sessionId,
      userId: gate.ctx.userId,
      userDisplayName: gate.ctx.userId,
      takeover,
    });
    if (!claim.ok) {
      const holder = await getEditorSession(fileId);
      return NextResponse.json(
        {
          error: "already_open",
          message: "Ce document est déjà ouvert dans un autre onglet.",
          holder,
          fileId,
        },
        { status: 409 },
      );
    }
  }

  const versions = resolved.isOwner
    ? await listOfficeVersions(resolved.ownerUserId, fileId)
    : [];

  return NextResponse.json({
    fileId,
    fileName: resolved.fileName,
    kind: resolved.kind,
    canWrite: resolved.canWrite,
    isOwner: resolved.isOwner,
    versions,
    mime: OFFICE_KIND_META[resolved.kind].mime,
  });
}

export async function POST(req: NextRequest) {
  const gate = await requireModule("documents");
  if (!gate.ok) return gate.response;
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  let body: Record<string, unknown>;
  try {
    body = (await req.json()) as Record<string, unknown>;
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const action = String(body.action || "");
  const scope = parseScope(String(body.scope || ""));
  const relPath = String(body.path || "");
  if (!scope || !relPath) {
    return NextResponse.json({ error: "Paramètres manquants." }, { status: 400 });
  }

  const resolved = await resolveOfficeStorage({
    userId: gate.ctx.userId,
    scope,
    shareId: body.shareId ? String(body.shareId) : null,
    fileShareId: body.fileShareId ? String(body.fileShareId) : null,
    relPath,
  });
  if (!resolved.ok) return NextResponse.json({ error: resolved.error }, { status: 403 });

  const fileId = makeOfficeFileId({
    etablissementId: tenant.ctx.etablissementId,
    scope,
    shareId: body.shareId ? String(body.shareId) : null,
    fileShareId: body.fileShareId ? String(body.fileShareId) : null,
    relPath,
  });

  if (action === "restore") {
    if (!resolved.isOwner) {
      return NextResponse.json(
        { error: "Seul le propriétaire peut restaurer une version." },
        { status: 403 },
      );
    }
    const versionId = String(body.versionId || "");
    if (!versionId) return NextResponse.json({ error: "Version manquante." }, { status: 400 });
    const restored = await restoreOfficeVersion({
      ownerUserId: resolved.ownerUserId,
      fileId,
      versionId,
      targetStorageKey: resolved.storageKey,
      contentType: OFFICE_KIND_META[resolved.kind].mime,
    });
    if (!restored.ok) return NextResponse.json({ error: restored.error }, { status: 400 });
    return NextResponse.json({ success: true });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
