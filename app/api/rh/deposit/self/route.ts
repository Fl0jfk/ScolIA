import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { depositRhSelfDocument } from "@/app/lib/rh/rh-deposit-self";

export const maxDuration = 60;

/** Dépôt personnel — identité via Better-Auth, classement direct sur OneDrive RH. */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });

  try {
    const form = await req.formData();
    const file = form.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier manquant." }, { status: 400 });
    }

    const bytes = new Uint8Array(await file.arrayBuffer());
    const result = await depositRhSelfDocument({
      externalUserId: user.id,
      email: user.primaryEmailAddress?.emailAddress || user.emailAddresses[0]?.emailAddress || "",
      uploadedBy: user.fullName || user.firstName || user.id,
      fileName: file.name,
      bytes,
      contentType: file.type || "application/pdf",
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }

    return NextResponse.json({
      ok: true,
      path: result.path,
      personnelId: result.personnelId,
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Dépôt impossible." },
      { status: 500 },
    );
  }
}
