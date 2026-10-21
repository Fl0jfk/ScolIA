import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import {
  canConseilFichesDialogue,
  canPublishFichesDialogue,
} from "@/app/lib/fiches-dialogue-access";
import { submitFdConseilDecision } from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

const ConseilSchema = z.object({
  etapeId: z.string().uuid(),
  payload: z.object({
    avis: z.enum(["favorable", "reserve", "defavorable", "autre"]),
    destinationProposee: z.string().optional(),
    optionsProposees: z.array(z.string()).optional(),
    motif: z.string().optional(),
    commentaire: z.string().optional(),
  }),
  signatures: z
    .array(
      z.object({
        role: z.enum(["professeur_principal", "direction"]),
        name: z.string().min(1),
        pngBase64: z.string().optional(),
        method: z.string().optional(),
      }),
    )
    .min(1),
  auteurLabel: z.string().optional(),
  /** false = brouillon PP ; true (défaut) = publication direction. */
  publish: z.boolean().optional(),
  /** true = rectification explicite d’une décision déjà publiée (avenant). */
  avenant: z.boolean().optional(),
  useSavedSignature: z.boolean().optional(),
});

export async function POST(req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canConseilFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const body = ConseilSchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }

  const publish = body.data.publish !== false;
  if (publish && !canPublishFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json(
      { error: "Seule la direction peut publier un avis ou une décision." },
      { status: 403 },
    );
  }
  let signatures = body.data.signatures;

  if (body.data.useSavedSignature) {
    const { loadUserSignatureBytes } = await import("@/app/lib/user-signature-store");
    const bytes = await loadUserSignatureBytes(scope.ctx.authUserId);
    if (bytes?.length) {
      const pngBase64 = Buffer.from(bytes).toString("base64");
      signatures = signatures.map((s) =>
        s.pngBase64
          ? s
          : {
              ...s,
              pngBase64,
              method: s.method || "saved",
            },
      );
    }
  }

  if (publish && !signatures.some((s) => s.role === "direction")) {
    return NextResponse.json(
      { error: "La publication nécessite une signature direction." },
      { status: 400 },
    );
  }

  if (body.data.avenant) {
    const motif = body.data.payload.motif?.trim() || body.data.payload.commentaire?.trim();
    if (!motif) {
      return NextResponse.json(
        { error: "Un avenant nécessite un motif explicite (rectification tracée)." },
        { status: 400 },
      );
    }
  }

  const result = await submitFdConseilDecision({
    etablissementId: scope.ctx.etablissementId,
    ficheId: id,
    etapeId: body.data.etapeId,
    payload: body.data.payload,
    auteurUserId: scope.ctx.authUserId,
    auteurLabel: body.data.auteurLabel ?? appUser.user.name,
    signatures,
    publish,
    avenant: body.data.avenant,
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json({ ok: true, published: result.published });
}
