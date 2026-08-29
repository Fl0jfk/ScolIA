import { NextResponse } from "next/server";
import { z } from "zod";
import {
  resolveFdToken,
  resolveFdTokenBySecureCode,
} from "@/app/lib/fiches-dialogue-tokens";
import {
  getFdPublicContext,
  submitFdAcceptation,
  submitFdFamilleReponse,
} from "@/app/lib/fiches-dialogue-workflow";

export async function GET(req: Request) {
  const url = new URL(req.url);
  const token = url.searchParams.get("token")?.trim() || "";
  if (!token) {
    return NextResponse.json({ error: "Token manquant." }, { status: 400 });
  }
  const ctx = await getFdPublicContext(token);
  if (!ctx) {
    return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
  }

  return NextResponse.json({
    fiche: {
      id: ctx.fiche.id,
      eleveNom: ctx.fiche.eleveNom,
      elevePrenom: ctx.fiche.elevePrenom,
      classeActuelle: ctx.fiche.classeActuelle,
      optionsActuelles: ctx.fiche.optionsActuelles,
      statut: ctx.fiche.statut,
    },
    campagne: {
      id: ctx.campagne.id,
      label: ctx.campagne.label,
      anneeLabel: ctx.campagne.anneeLabel,
      catalogue: ctx.campagne.catalogue,
      appelConfig: ctx.campagne.appelConfig,
      delaiFamilleJours: ctx.campagne.delaiFamilleJours,
    },
    etape: {
      id: ctx.etape.id,
      kind: ctx.etape.kind,
      label: ctx.etape.label,
      description: ctx.etape.description,
      gelee: ctx.etape.gelee,
    },
    reponses: ctx.reponses.map((r) => ({
      etapeId: r.etapeId,
      auteurRole: r.auteurRole,
      payload: r.payload,
      submittedAt: r.submittedAt,
    })),
  });
}

const ResolveSchema = z.object({
  action: z.literal("resolve_code"),
  email: z.string().email(),
  code: z.string().min(4).max(12),
});

const SubmitSchema = z.object({
  action: z.literal("submit"),
  token: z.string().min(10),
  auteurLabel: z.string().optional(),
  signature: z
    .object({
      name: z.string().min(1),
      pngBase64: z.string().optional(),
      method: z.string().optional(),
      email: z.string().email().optional(),
    })
    .optional(),
  kind: z.enum(["saisie", "acceptation"]),
  payload: z.record(z.string(), z.unknown()),
});

export async function POST(req: Request) {
  const raw = await req.json().catch(() => null);

  const resolve = ResolveSchema.safeParse(raw);
  if (resolve.success) {
    const row = await resolveFdTokenBySecureCode(resolve.data.email, resolve.data.code);
    if (!row) {
      return NextResponse.json({ error: "Code invalide." }, { status: 404 });
    }
    return NextResponse.json({ token: row.token });
  }

  const submit = SubmitSchema.safeParse(raw);
  if (!submit.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }

  const tokenRow = await resolveFdToken(submit.data.token);
  if (!tokenRow) {
    return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
  }
  const ctx = await getFdPublicContext(submit.data.token);
  if (!ctx) {
    return NextResponse.json({ error: "Contexte introuvable." }, { status: 404 });
  }
  if (ctx.etape.gelee) {
    return NextResponse.json({ error: "Cette étape est figée." }, { status: 409 });
  }

  if (submit.data.kind === "acceptation" || ctx.etape.kind === "acceptation_famille") {
    const accepte = Boolean(submit.data.payload.accepte);
    const motifRefus =
      typeof submit.data.payload.motifRefus === "string"
        ? submit.data.payload.motifRefus
        : undefined;
    const result = await submitFdAcceptation({
      etablissementId: tokenRow.etablissementId,
      ficheId: ctx.fiche.id,
      etapeId: ctx.etape.id,
      payload: { accepte, motifRefus },
      auteurLabel: submit.data.auteurLabel,
      signature: submit.data.signature,
    });
    if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
    return NextResponse.json({ ok: true });
  }

  const values: Record<string, string | string[] | boolean | null> = {};
  for (const [k, v] of Object.entries(submit.data.payload.values ?? submit.data.payload)) {
    if (k === "comment" || k === "forceMalgreAvis" || k === "accepte" || k === "motifRefus") {
      continue;
    }
    if (
      typeof v === "string" ||
      typeof v === "boolean" ||
      v === null ||
      (Array.isArray(v) && v.every((x) => typeof x === "string"))
    ) {
      values[k] = v as string | string[] | boolean | null;
    }
  }

  const result = await submitFdFamilleReponse({
    etablissementId: tokenRow.etablissementId,
    ficheId: ctx.fiche.id,
    etapeId: ctx.etape.id,
    payload: {
      values,
      comment:
        typeof submit.data.payload.comment === "string"
          ? submit.data.payload.comment
          : undefined,
      forceMalgreAvis: Boolean(submit.data.payload.forceMalgreAvis),
    },
    auteurLabel: submit.data.auteurLabel,
    signature: submit.data.signature,
  });

  if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
  return NextResponse.json({ ok: true });
}
