import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { deleteSanta, readSantaFor, updateSantaBeforeDraw, updateSantaTexts } from "@/app/lib/secret-santa-db";
import { buildSantaView, parseSantaInput, parseSantaTextUpdate, receiverOf } from "@/app/lib/secret-santa";
import { listAvatarVersions } from "@/app/lib/avatar-db";

export const dynamic = "force-dynamic";

type Params = { params: Promise<{ id: string }> };

export async function GET(_req: Request, { params }: Params) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const now = new Date();
    const { santa, participants } = await readSantaFor(auth.ctx.etablissementId, id, auth.ctx.userId, now);
    const view = buildSantaView(santa, participants, auth.ctx.userId, now);
    const receiver = view.mine ? receiverOf(participants, auth.ctx.userId) : null;
    if (view.mine && receiver && (await listAvatarVersions([receiver.userId])).size) {
      // La version force le rechargement après un changement, sans désigner la personne.
      view.mine.photoUrl = `/api/secret-santa/${encodeURIComponent(id)}/photo?v=${santa.version}`;
    }
    return NextResponse.json(view, {
      headers: { "Cache-Control": "no-store" },
    });
  } catch (error) {
    return santaErrorResponse(error);
  }
}

export async function PATCH(req: Request, { params }: Params) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const body = (await req.json().catch(() => null)) as { textsOnly?: unknown } | null;
    if (body?.textsOnly === true) {
      await updateSantaTexts(auth.ctx.etablissementId, id, auth.ctx.userId, parseSantaTextUpdate(body));
    } else {
      await updateSantaBeforeDraw(
        auth.ctx.etablissementId,
        id,
        auth.ctx.userId,
        parseSantaInput(body, new Date()),
      );
    }
    return NextResponse.json({ success: true });
  } catch (error) {
    return santaErrorResponse(error);
  }
}

export async function DELETE(_req: Request, { params }: Params) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    await deleteSanta(auth.ctx.etablissementId, id, auth.ctx.userId);
    return NextResponse.json({ success: true });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
