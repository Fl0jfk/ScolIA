import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getFamilleMessagingSettings } from "@/app/lib/famille-messaging-db";
import { canInitiateFromMatrix } from "@/app/lib/famille-messaging-matrix";
import { storeFamilleAttachmentFile } from "@/app/lib/famille-messaging-upload";

export const maxDuration = 60;

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const settings = await getFamilleMessagingSettings(etabId);
  if (!canInitiateFromMatrix(roles, settings, { orgAdmin: user?.orgAdmin })) {
    return NextResponse.json({ error: "Action non autorisée (matrice)." }, { status: 403 });
  }

  try {
    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Aucun fichier trouvé." }, { status: 400 });
    }
    const threadOrTemp = String(formData.get("threadId") || "draft").trim() || "draft";
    const att = await storeFamilleAttachmentFile({
      etablissementId: etabId,
      threadOrTempId: threadOrTemp,
      file,
    });
    return NextResponse.json({ attachment: att });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Upload impossible." },
      { status: 400 },
    );
  }
}
