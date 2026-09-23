import { NextRequest, NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { getFamilleMessagingSettings } from "@/app/lib/famille-messaging-db";
import { storeFamilleAttachmentFile } from "@/app/lib/famille-messaging-upload";

export const maxDuration = 60;

export async function POST(req: NextRequest) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;
  const settings = await getFamilleMessagingSettings(gate.ctx.etablissementId);
  if (!settings.allowParentAttachments) {
    return NextResponse.json(
      { error: "Les pièces jointes parents sont désactivées." },
      { status: 403 },
    );
  }
  try {
    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Aucun fichier trouvé." }, { status: 400 });
    }
    const threadOrTemp = String(formData.get("threadId") || "draft").trim() || "draft";
    const att = await storeFamilleAttachmentFile({
      etablissementId: gate.ctx.etablissementId,
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
