import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getBucketName } from "@/app/lib/s3-storage";
import {
  buildCollaboraEditorUrl,
  getCollaboraPublicUrl,
  getWopiHostUrl,
  makeOfficeFileId,
  resolveOfficeStorage,
  signWopiToken,
  claimEditorSession,
  getEditorSession,
} from "@/app/lib/office-wopi";
import { isUntitledRootDraft } from "@/app/lib/office-types";
import type { OfficeScope } from "@/app/lib/office-types";
import { touchOfficeRecent } from "@/app/lib/office-recents";

function parseScope(raw: string | null): OfficeScope | null {
  const s = String(raw || "");
  return s === "personal" || s === "shared" || s === "fileshare" ? s : null;
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

  const scope = parseScope(String(body.scope || ""));
  const relPath = String(body.path || "");
  const shareId = body.shareId ? String(body.shareId) : null;
  const fileShareId = body.fileShareId ? String(body.fileShareId) : null;
  const sessionId = String(body.sessionId || crypto.randomUUID());
  const takeover = body.takeover === true;

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

  const user = await safeCurrentUser();
  const displayName =
    [user?.firstName, user?.lastName].filter(Boolean).join(" ").trim() ||
    user?.emailAddresses?.[0]?.emailAddress ||
    gate.ctx.userId;

  const claim = await claimEditorSession({
    fileId,
    sessionId,
    userId: gate.ctx.userId,
    userDisplayName: displayName,
    takeover,
  });
  if (!claim.ok) {
    return NextResponse.json(
      {
        error: "already_open",
        message: "Ce document est déjà ouvert dans un autre onglet.",
        holder: await getEditorSession(fileId),
        sessionId,
        fileId,
      },
      { status: 409 },
    );
  }

  const dataBucket = await getBucketName();
  const token = signWopiToken({
    fileId,
    userId: gate.ctx.userId,
    userDisplayName: displayName,
    etablissementId: tenant.ctx.etablissementId,
    dataBucket,
    storageKey: resolved.storageKey,
    fileName: resolved.fileName,
    canWrite: resolved.canWrite,
    isOwner: resolved.isOwner,
    ownerUserId: resolved.ownerUserId,
    kind: resolved.kind,
    scope,
    shareId,
    fileShareId,
    relPath,
    sessionId,
  });

  const wopiSrc = `${getWopiHostUrl()}/api/wopi/files/${fileId}`;
  const editorUrl = buildCollaboraEditorUrl({ wopiSrc, accessToken: token });
  const collaboraConfigured = Boolean(getCollaboraPublicUrl());

  await touchOfficeRecent(gate.ctx.userId, {
    kind: resolved.kind,
    scope,
    shareId,
    fileShareId,
    relPath,
    fileName: resolved.fileName,
    sharedLabel: resolved.shareLabel,
  });

  const draft =
    scope === "personal" &&
    !shareId &&
    !fileShareId &&
    (body.draft === true || body.draft === "1" || isUntitledRootDraft(relPath, resolved.kind));

  return NextResponse.json({
    fileId,
    sessionId,
    accessToken: token,
    wopiSrc,
    editorUrl,
    collaboraConfigured,
    fileName: resolved.fileName,
    kind: resolved.kind,
    canWrite: resolved.canWrite,
    isOwner: resolved.isOwner,
    needsPlaceOnClose: draft,
    scope,
    shareId,
    fileShareId,
    relPath,
  });
}
