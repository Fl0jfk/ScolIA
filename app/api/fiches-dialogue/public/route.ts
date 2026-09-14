import { NextResponse } from "next/server";
import { z } from "zod";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  assertFdParentEmailOnFiche,
  identifyFdFicheByIdentity,
} from "@/app/lib/fiches-dialogue-identity";
import { notifyFdParentOtp } from "@/app/lib/fiches-dialogue-notify";
import {
  createFdAccessToken,
  resolveFdToken,
  resolveFdTokenBySecureCode,
} from "@/app/lib/fiches-dialogue-tokens";
import {
  getFdFiche,
  getFdPublicContext,
  isFdEtapeOpenForFamille,
  listFdEtapes,
  submitFdAcceptation,
  submitFdFamilleReponse,
  submitFdParentAccord,
} from "@/app/lib/fiches-dialogue-workflow";
import { normalizeParentEmail } from "@/app/lib/eleves-parent-emails";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const identifyLimiter = createMemoryRateLimiter({ windowMs: 15 * 60 * 1000, max: 20 });
const globalLimiter = createMemoryRateLimiter({ windowMs: 10 * 60 * 1000, max: 40 });
const otpLimiter = createMemoryRateLimiter({ windowMs: 2 * 60 * 1000, max: 1 });

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

  const open = isFdEtapeOpenForFamille(ctx.etape);
  const appel = {
    ...ctx.campagne.appelConfig,
    contactPpLabel:
      ctx.campagne.appelConfig?.contactPpLabel ||
      ctx.campagne.contactPpLabel ||
      "via École Directe",
  };

  return NextResponse.json({
    fiche: {
      id: ctx.fiche.id,
      eleveNom: ctx.fiche.eleveNom,
      elevePrenom: ctx.fiche.elevePrenom,
      classeActuelle: ctx.fiche.classeActuelle,
      eleveDateNaissance: ctx.fiche.eleveDateNaissance,
      elevePhotoKey: ctx.fiche.elevePhotoKey,
      optionsActuelles: ctx.fiche.optionsActuelles,
      statut: ctx.fiche.statut,
      parentAccord: ctx.fiche.parentAccord,
      acceptation: ctx.fiche.acceptation,
    },
    campagne: {
      id: ctx.campagne.id,
      label: ctx.campagne.label,
      anneeLabel: ctx.campagne.anneeLabel,
      catalogue: ctx.campagne.catalogue,
      appelConfig: appel,
      starterMode: ctx.campagne.starterMode,
      delaiFamilleJours: ctx.campagne.delaiFamilleJours,
      contactPpLabel: ctx.campagne.contactPpLabel,
    },
    etape: {
      id: ctx.etape.id,
      kind: ctx.etape.kind,
      label: ctx.etape.label,
      description: ctx.etape.description,
      gelee: ctx.etape.gelee,
      opensAt: ctx.etape.opensAt,
      closesAt: ctx.etape.closesAt,
      openForFamille: open,
    },
    reponses: ctx.reponses.map((r) => ({
      etapeId: r.etapeId,
      auteurRole: r.auteurRole,
      payload: r.payload,
      submittedAt: r.submittedAt,
    })),
  });
}

const IdentifySchema = z.object({
  action: z.literal("identify"),
  nom: z.string().min(1).max(120),
  prenom: z.string().min(1).max(120),
  dateNaissance: z.string().min(4).max(32),
  classe: z.string().max(64).optional(),
  campagneId: z.string().uuid().optional(),
});

const RequestCodeSchema = z.object({
  action: z.literal("request_code"),
  ficheId: z.string().uuid(),
  email: z.string().email(),
});

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

const ParentAccordSchema = z.object({
  action: z.literal("parent_accord"),
  token: z.string().min(10),
  decision: z.enum(["confirme", "contredit"]),
  motif: z.string().max(2000).optional(),
  conflictValues: z.record(z.string(), z.unknown()).optional(),
});

const GENERIC_FAIL =
  "Si cet élève est le vôtre, un code a été envoyé aux adresses que nous avons pour la famille. Sinon, vérifiez les informations saisies.";

export async function POST(req: Request) {
  const raw = await req.json().catch(() => null);
  const ip = clientIpFromRequest(req);
  if (!(await globalLimiter.allow(`fd-public:${ip}`))) {
    return NextResponse.json({ error: "Trop de tentatives. Réessayez plus tard." }, { status: 429 });
  }

  const identify = IdentifySchema.safeParse(raw);
  if (identify.success) {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) {
      return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
    }
    if (!(await identifyLimiter.allow(`fd-identify:${ip}`))) {
      return NextResponse.json({ error: "Trop de tentatives. Réessayez plus tard." }, { status: 429 });
    }

    const result = await identifyFdFicheByIdentity({
      etablissementId: etabId,
      nom: identify.data.nom,
      prenom: identify.data.prenom,
      dateNaissance: identify.data.dateNaissance,
      classe: identify.data.classe,
      campagneId: identify.data.campagneId,
    });

    if (!result.ok) {
      return NextResponse.json({
        ok: true,
        sent: false,
        message: GENERIC_FAIL,
      });
    }
    if ("needsClass" in result && result.needsClass) {
      return NextResponse.json({
        ok: true,
        needsClass: true,
        classes: result.classes,
        message: "Plusieurs élèves correspondent. Indiquez la classe.",
      });
    }
    if (!("match" in result)) {
      return NextResponse.json({ ok: true, sent: false, message: GENERIC_FAIL });
    }

    return NextResponse.json({
      ok: true,
      match: {
        ficheId: result.match.ficheId,
        campagneLabel: result.match.campagneLabel,
        eleveNom: result.match.eleveNom,
        elevePrenom: result.match.elevePrenom,
        classeActuelle: result.match.classeActuelle,
        dateNaissance: result.match.dateNaissance,
        optionsActuelles: result.match.optionsActuelles,
        photoKey: result.match.photoKey,
        maskedEmails: result.match.maskedEmails.map((m) => ({
          masked: m.masked,
          // email réel uniquement pour que le parent choisisse — transmis chiffré côté UI
          // mais on ne l’expose qu’après match : nécessaire pour request_code
          email: m.email,
        })),
      },
      message:
        result.match.maskedEmails.length > 0
          ? "Choisissez votre adresse e-mail pour recevoir un code d’accès."
          : "Aucune adresse e-mail n’est enregistrée pour cette famille. Contactez l’accueil.",
    });
  }

  const requestCode = RequestCodeSchema.safeParse(raw);
  if (requestCode.success) {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) {
      return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
    }
    const email = normalizeParentEmail(requestCode.data.email);
    const allowed = await assertFdParentEmailOnFiche({
      etablissementId: etabId,
      ficheId: requestCode.data.ficheId,
      email,
    });
    // Toujours même réponse (pas d’énumération)
    if (!allowed) {
      return NextResponse.json({
        ok: true,
        sent: true,
        message: GENERIC_FAIL,
      });
    }

    const fiche = await getFdFiche(etabId, requestCode.data.ficheId);
    if (!fiche?.etapeCouranteId) {
      return NextResponse.json({ ok: true, sent: true, message: GENERIC_FAIL });
    }
    const etapes = await listFdEtapes(etabId, fiche.campagneId);
    const etape = etapes.find((e) => e.id === fiche.etapeCouranteId) ?? etapes[0];
    if (!etape) {
      return NextResponse.json({ ok: true, sent: true, message: GENERIC_FAIL });
    }

    const sendOk = await otpLimiter.allow(`fd-otp:${fiche.id}:${email}`);
    if (!sendOk) {
      return NextResponse.json({
        ok: true,
        sent: true,
        message: "Un code a déjà été envoyé. Vérifiez votre boîte ou attendez 2 minutes.",
      });
    }

    const tokenRow = await createFdAccessToken({
      etablissementId: etabId,
      ficheId: fiche.id,
      etapeId: etape.id,
      email,
      purpose: etape.kind === "acceptation_famille" ? "acceptation" : "saisie",
      expiresInMinutes: 30,
    });

    await notifyFdParentOtp({
      to: email,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      code: tokenRow.secureCode || "",
      expiresMinutes: 30,
    });

    return NextResponse.json({
      ok: true,
      sent: true,
      message: "Si cette adresse est la vôtre, un code à 6 chiffres vient d’être envoyé.",
    });
  }

  const resolve = ResolveSchema.safeParse(raw);
  if (resolve.success) {
    const row = await resolveFdTokenBySecureCode(resolve.data.email, resolve.data.code);
    if (!row) {
      return NextResponse.json({ error: "Code invalide." }, { status: 404 });
    }
    return NextResponse.json({ token: row.token });
  }

  const submit = SubmitSchema.safeParse(raw);
  if (submit.success) {
    const tokenRow = await resolveFdToken(submit.data.token);
    if (!tokenRow) {
      return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
    }
    const ctx = await getFdPublicContext(submit.data.token);
    if (!ctx) {
      return NextResponse.json({ error: "Contexte introuvable." }, { status: 404 });
    }
    if (!isFdEtapeOpenForFamille(ctx.etape)) {
      return NextResponse.json(
        { error: "Cette étape n’est plus modifiable (date limite ou gel)." },
        { status: 409 },
      );
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
      return NextResponse.json({ ok: true, refused: !accepte });
    }

    const values: Record<string, string | string[] | boolean | null> = {};
    const rawValues =
      submit.data.payload.values && typeof submit.data.payload.values === "object"
        ? (submit.data.payload.values as Record<string, unknown>)
        : submit.data.payload;
    for (const [k, v] of Object.entries(rawValues)) {
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

    const etabRaw = submit.data.payload.etablissementsVoeux;
    const etablissementsVoeux = Array.isArray(etabRaw)
      ? etabRaw
          .map((item, idx) => {
            if (!item || typeof item !== "object") return null;
            const row = item as Record<string, unknown>;
            const codeRne = typeof row.codeRne === "string" ? row.codeRne : "";
            const label = typeof row.label === "string" ? row.label : codeRne;
            if (!codeRne) return null;
            return {
              rang: typeof row.rang === "number" ? row.rang : idx + 1,
              codeRne,
              label,
              chezNous: Boolean(row.chezNous),
            };
          })
          .filter((x): x is NonNullable<typeof x> => Boolean(x))
      : undefined;

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
        etablissementsVoeux,
      },
      auteurLabel: submit.data.auteurLabel,
      signature: {
        ...submit.data.signature,
        name: submit.data.signature?.name || submit.data.auteurLabel || "Famille",
        email:
          submit.data.signature?.email ||
          tokenRow.email ||
          undefined,
      },
    });

    if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
    return NextResponse.json({ ok: true, needsParent2: Boolean(result.needsParent2) });
  }

  const parentAccord = ParentAccordSchema.safeParse(raw);
  if (parentAccord.success) {
    const tokenRow = await resolveFdToken(parentAccord.data.token);
    if (!tokenRow?.email) {
      return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
    }
    const conflictValues: Record<string, string | string[] | boolean | null> = {};
    if (parentAccord.data.conflictValues) {
      for (const [k, v] of Object.entries(parentAccord.data.conflictValues)) {
        if (
          typeof v === "string" ||
          typeof v === "boolean" ||
          v === null ||
          (Array.isArray(v) && v.every((x) => typeof x === "string"))
        ) {
          conflictValues[k] = v as string | string[] | boolean | null;
        }
      }
    }
    const result = await submitFdParentAccord({
      etablissementId: tokenRow.etablissementId,
      ficheId: tokenRow.ficheId,
      email: tokenRow.email,
      decision: parentAccord.data.decision,
      motif: parentAccord.data.motif,
      conflictValues:
        Object.keys(conflictValues).length > 0 ? conflictValues : undefined,
    });
    if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
    return NextResponse.json({ ok: true });
  }

  return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
}
