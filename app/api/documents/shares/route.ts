import { NextRequest, NextResponse } from "next/server";
import {
  createSharedFolder,
  listAccessibleShares,
  updateSharedMembers,
} from "@/app/lib/documents-cloud";
import { notifySharedFolderInvites } from "@/app/lib/documents-notify";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";

function scheduleSharedFolderInviteMails(params: {
  shareId: string;
  shareName: string;
  inviteeUserIds: string[];
}) {
  if (params.inviteeUserIds.length === 0) return;

  void (async () => {
    const user = await safeCurrentUser();
    const email =
      user?.emailAddresses.find((e) => e.id === user.primaryEmailAddressId)?.emailAddress ??
      user?.emailAddresses[0]?.emailAddress ??
      null;
    await notifySharedFolderInvites({
      shareId: params.shareId,
      shareName: params.shareName,
      inviteeUserIds: params.inviteeUserIds,
      inviter: {
        firstName: user?.firstName,
        lastName: user?.lastName,
        email,
      },
    });
  })().catch((e) => console.error("[documents] notify shared folder invite:", e));
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const shares = await listAccessibleShares(gate.ctx.userId);
  return NextResponse.json({
    shares: shares.map((s) => ({
      ...s,
      isOwner: s.ownerId === gate.ctx.userId,
    })),
  });
}

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const name = String(body.name ?? "").trim();
    const memberIds = Array.isArray(body.memberIds) ? body.memberIds.map(String) : [];

    if (!name) {
      return NextResponse.json({ error: "Nom du dossier requis." }, { status: 400 });
    }

    const meta = await createSharedFolder(gate.ctx.userId, name, memberIds);
    scheduleSharedFolderInviteMails({
      shareId: meta.id,
      shareName: meta.name,
      inviteeUserIds: meta.memberIds,
    });
    return NextResponse.json({ success: true, share: meta });
  } catch (e) {
    console.error("[documents/shares] POST", e);
    return NextResponse.json(
      {
        error:
          e instanceof Error && /Postgres|ENT_CORE_DB|DATABASE_URL/i.test(e.message)
            ? "Impossible de créer le dossier partagé (stockage indisponible)."
            : e instanceof Error
              ? e.message
              : "Création impossible.",
      },
      { status: 500 },
    );
  }
}

export async function PATCH(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const body = await req.json();
  const shareId = String(body.shareId ?? "").trim();
  const memberIds = Array.isArray(body.memberIds) ? body.memberIds.map(String) : [];

  if (!shareId) {
    return NextResponse.json({ error: "shareId requis." }, { status: 400 });
  }

  const result = await updateSharedMembers(gate.ctx.userId, shareId, memberIds);
  if (!result.ok) return NextResponse.json({ error: result.error }, { status: 403 });

  scheduleSharedFolderInviteMails({
    shareId: result.meta.id,
    shareName: result.meta.name,
    inviteeUserIds: result.addedMemberIds,
  });

  return NextResponse.json({ success: true, share: result.meta });
}
