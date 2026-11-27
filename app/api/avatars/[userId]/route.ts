import { NextResponse } from "next/server";
import { avatarErrorResponse, requireAvatarContext, resolveAvatarTarget } from "@/app/lib/avatar-access";
import { avatarResponse, deleteAvatar, readAvatar, saveAvatar } from "@/app/lib/avatar-db";
import { AVATAR_MAX_INPUT_BYTES, AvatarError, toAvatarJpeg } from "@/app/lib/avatar-image";

export const dynamic = "force-dynamic";

type Params = { params: Promise<{ userId: string }> };

const notFound = () => NextResponse.json({ error: "Pas de photo." }, { status: 404 });

/** Visible du personnel du même établissement seulement. */
export async function GET(_req: Request, { params }: Params) {
  try {
    const context = await requireAvatarContext();
    if (!context.ok) return context.response;
    const { userId } = await params;
    const target = userId === "me" ? context.actor.userId : userId;
    if (target !== context.actor.userId && !context.staff.some((m) => m.userId === target)) return notFound();
    const avatar = await readAvatar(target);
    return avatar ? avatarResponse(avatar.image) : notFound();
  } catch (error) {
    return avatarErrorResponse(error);
  }
}

export async function PUT(req: Request, { params }: Params) {
  try {
    const context = await requireAvatarContext();
    if (!context.ok) return context.response;
    const target = resolveAvatarTarget(context, (await params).userId);
    // Refus avant de lire en mémoire un envoi démesuré ou de taille non annoncée.
    const length = Number(req.headers.get("content-length") || 0);
    if (!length || length > AVATAR_MAX_INPUT_BYTES + 64 * 1024) {
      throw new AvatarError("Photo trop lourde (8 Mo au maximum).", 413);
    }
    const file = (await req.formData().catch(() => null))?.get("photo");
    if (!(file instanceof Blob)) throw new AvatarError("Aucune photo reçue.");
    const image = await toAvatarJpeg(Buffer.from(await file.arrayBuffer()));
    return NextResponse.json({ photoUrl: await saveAvatar(target, image, context.actor.userId) });
  } catch (error) {
    return avatarErrorResponse(error);
  }
}

export async function DELETE(_req: Request, { params }: Params) {
  try {
    const context = await requireAvatarContext();
    if (!context.ok) return context.response;
    await deleteAvatar(resolveAvatarTarget(context, (await params).userId));
    return NextResponse.json({ success: true });
  } catch (error) {
    return avatarErrorResponse(error);
  }
}
