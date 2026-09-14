import { getJson, putJson, getObjectBytes } from "@/app/lib/s3-storage";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import { wizardStep } from "@/app/lib/brain-ai/wizard";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import {
  canCreatePhotocopiesDemand,
  canViewPhotocopiesDemand,
  getPhotocopiesRoleFlags,
} from "@/app/lib/photocopies-couleur-access";
import {
  establishmentChoiceOptions,
  isAnyDirectionRole,
  matchEstablishment,
} from "@/app/lib/establishment-catalog";
import {
  PHOTOCOPIES_MAX_DOCUMENTS,
  getPhotocopieDocuments,
  hasPhotocopieDocuments,
  photocopieDocumentFields,
  type PhotoCopieDocument,
  type PhotoCopieRecord,
} from "@/app/lib/photocopies-couleur-types";

const INDEX_KEY = "photocopies-couleur/index.json";

function isValidDocumentKey(key: string): boolean {
  return (
    (key.startsWith("photocopies-couleur/uploads/") || key.startsWith("brain-ai/uploads/")) &&
    !key.includes("..")
  );
}

function parseDocumentsFromArgs(args: Record<string, unknown>): {
  ok: true;
  docs: PhotoCopieDocument[];
} | { ok: false; error: string } {
  const rawList = Array.isArray(args.documents) ? args.documents : null;
  const docs: PhotoCopieDocument[] = [];

  if (rawList) {
    if (rawList.length > PHOTOCOPIES_MAX_DOCUMENTS) {
      return { ok: false, error: `Maximum ${PHOTOCOPIES_MAX_DOCUMENTS} PDF par demande.` };
    }
    for (const entry of rawList) {
      if (!entry || typeof entry !== "object") continue;
      const row = entry as Record<string, unknown>;
      const key = String(row.key || row.documentKey || "").trim();
      const fileName = String(row.fileName || row.documentFileName || "").trim();
      const contentType = String(row.contentType || row.documentContentType || "application/pdf").trim();
      if (!key && !fileName) continue;
      if (!key || !isValidDocumentKey(key)) {
        return { ok: false, error: "Document joint invalide." };
      }
      if (!fileName) {
        return { ok: false, error: "Nom du fichier PDF requis avec la pièce jointe." };
      }
      docs.push({ key, fileName, contentType: contentType || "application/pdf" });
    }
  } else {
    const documentKey = String(args.documentKey || "").trim();
    const documentFileName = String(args.documentFileName || "").trim();
    const documentContentType = String(args.documentContentType || "application/pdf").trim();
    if (documentKey) {
      if (!isValidDocumentKey(documentKey)) {
        return { ok: false, error: "Document joint invalide." };
      }
      if (!documentFileName) {
        return { ok: false, error: "Nom du fichier PDF requis avec la pièce jointe." };
      }
      docs.push({
        key: documentKey,
        fileName: documentFileName,
        contentType: documentContentType || "application/pdf",
      });
    }
  }

  return { ok: true, docs: docs.slice(0, PHOTOCOPIES_MAX_DOCUMENTS) };
}

async function loadEtabOptions() {
  const bundle = await loadAppConfig();
  return {
    establishments: bundle.establishments,
    choices: establishmentChoiceOptions(bundle.establishments),
  };
}

async function getIndex(): Promise<PhotoCopieRecord[]> {
  const hit = await getJson<PhotoCopieRecord[]>(INDEX_KEY);
  return hit?.data ?? [];
}

export async function handleListPhotocopies(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  const { establishments } = await loadEtabOptions();
  const f = getPhotocopiesRoleFlags(ctx.roles);
  if (!canCreatePhotocopiesDemand(ctx.roles) && !f.isDirection && !isAnyDirectionRole(ctx.roles)) {
    return { ok: false, error: "Accès réservé aux photocopies couleur.", code: "MODULE_FORBIDDEN" };
  }

  const statusFilter = typeof args.status === "string" ? args.status.trim().toUpperCase() : "";
  const limit = Math.min(Math.max(Number(args.limit) || 15, 1), 40);
  const all = await getIndex();
  let items = all
    .filter((r) => canViewPhotocopiesDemand(r, ctx.userId!, ctx.roles, establishments))
    .sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
  if (statusFilter) {
    items = items.filter((r) => r.status === statusFilter);
  }
  const brief = items.slice(0, limit).map((r) => {
    const docs = getPhotocopieDocuments(r);
    return {
      id: r.id,
      status: r.status,
      etablissement: r.etablissement,
      nombrePhotocopies: r.nombrePhotocopies,
      motif: r.motif.slice(0, 120),
      classesOuMatiere: r.classesOuMatiere,
      hasDocument: hasPhotocopieDocuments(r),
      documentCount: docs.length,
      createdAt: r.createdAt.slice(0, 10),
      mine: r.createdBy.userId === ctx.userId,
    };
  });

  return {
    ok: true,
    data: {
      items: brief,
      totalVisible: items.length,
      ctas: [{ label: "Ouvrir Photocopies couleur", href: "/photocopies-couleur" }],
    },
    summaryFr:
      brief.length === 0
        ? "Aucune demande de photocopies visible."
        : `${brief.length} demande(s) : ${brief
            .slice(0, 5)
            .map((i) => `${i.status} — ${i.nombrePhotocopies} ex. (${i.etablissement})`)
            .join(" · ")}.`,
  };
}

export async function handleCreatePhotocopie(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  if (!canCreatePhotocopiesDemand(ctx.roles)) {
    return {
      ok: false,
      error: "Seuls les enseignants, la vie scolaire et l'administratif peuvent créer une demande.",
      code: "MODULE_FORBIDDEN",
    };
  }
  if (!ctx.email) {
    return { ok: false, error: "Votre compte doit avoir une adresse e-mail." };
  }

  let etablissement = String(args.etablissement || "").trim();
  let motif = String(args.motif || "").trim();
  let classesOuMatiere = String(args.classesOuMatiere || "").trim();
  const nbRaw = args.nombrePhotocopies;
  const nb = Number(nbRaw);
  const parsedDocs = parseDocumentsFromArgs(args);
  if (!parsedDocs.ok) {
    return { ok: false, error: parsedDocs.error };
  }
  const docs = parsedDocs.docs;
  const documentFields = photocopieDocumentFields(docs);

  const total = 4;
  let step = 1;
  const draft = (): Record<string, unknown> => ({
    etablissement,
    motif,
    classesOuMatiere,
    ...(Number.isFinite(nb) && nb >= 1 ? { nombrePhotocopies: nb } : {}),
    ...documentFields,
  });

  const { establishments, choices } = await loadEtabOptions();
  const matched = matchEstablishment(establishments, etablissement);
  if (!matched) {
    return choicesResult(
      "create_photocopie_demand",
      "etablissement",
      wizardStep(step, total, "Demande de photocopies couleur — pour quel établissement ?"),
      choices,
      draft(),
    );
  }
  etablissement = matched.label;
  step += 1;

  if (!motif) {
    return choicesResult(
      "create_photocopie_demand",
      "motif",
      wizardStep(step, total, "Quel est le motif de la demande ?"),
      [],
      draft(),
      "text",
    );
  }
  step += 1;

  if (!classesOuMatiere) {
    return choicesResult(
      "create_photocopie_demand",
      "classesOuMatiere",
      wizardStep(step, total, "Classes / matière concernées ?"),
      [],
      draft(),
      "text",
    );
  }
  step += 1;

  if (!Number.isFinite(nb) || nb < 1 || nb > 1_000_000) {
    return choicesResult(
      "create_photocopie_demand",
      "nombrePhotocopies",
      wizardStep(step, total, "Combien d'exemplaires ? (nombre entier)"),
      [],
      draft(),
      "text",
    );
  }

  if (!ctx.confirmed) {
    const pdfLine =
      docs.length === 0
        ? `• PDF : aucun (vous pouvez encore en joindre jusqu'à ${PHOTOCOPIES_MAX_DOCUMENTS} via le trombone avant de confirmer)`
        : docs.length === 1
          ? `• PDF joint : ${docs[0].fileName}`
          : `• PDF joints (${docs.length}) : ${docs.map((d) => d.fileName).join(", ")}`;
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_photocopie_demand",
      args: {
        etablissement,
        motif,
        classesOuMatiere,
        nombrePhotocopies: nb,
        ...documentFields,
      },
      summaryFr:
        `Récapitulatif — ${nb} photocopie(s) couleur\n` +
        `• Établissement : ${etablissement}\n` +
        `• Classes / matière : ${classesOuMatiere}\n` +
        `• Motif : ${motif.slice(0, 160)}${motif.length > 160 ? "…" : ""}\n` +
        pdfLine,
    };
  }

  const record: PhotoCopieRecord = {
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    status: "EN_ATTENTE",
    createdBy: {
      userId: ctx.userId,
      name: [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || ctx.email,
      email: ctx.email,
    },
    etablissement,
    motif,
    classesOuMatiere,
    nombrePhotocopies: nb,
    ...documentFields,
  };

  const all = await getIndex();
  all.push(record);
  await putJson(INDEX_KEY, all);

  try {
    const bundle = await loadAppConfig();
    const est = getEstablishmentByLabel(bundle, etablissement);
    const dirEmail = est?.directorEmail || "";
    const dirName = est?.directorName || est?.label || etablissement;
    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (transporter && smtp && dirEmail) {
      const link = await tenantAbsolutePath("/photocopies-couleur");
      const attachments: Array<{ filename: string; content: Buffer; contentType: string }> = [];
      const usedNames = new Map<string, number>();
      for (const doc of getPhotocopieDocuments(record)) {
        const bytes = await getObjectBytes(doc.key);
        if (!bytes?.length) continue;
        const base = doc.fileName || "document.pdf";
        const count = (usedNames.get(base) ?? 0) + 1;
        usedNames.set(base, count);
        const filename =
          count === 1
            ? base
            : /\.pdf$/i.test(base)
              ? base.replace(/\.pdf$/i, `-${count}.pdf`)
              : `${base}-${count}`;
        attachments.push({
          filename,
          content: bytes,
          contentType: doc.contentType || "application/pdf",
        });
      }
      await transporter.sendMail({
        from: `"Demandes photocopies" <${smtp.user}>`,
        to: dirEmail,
        subject: `Photocopies couleur — nouvelle demande (${etablissement})`,
        text: [
          `Bonjour ${dirName},`,
          ``,
          `Demandeur : ${record.createdBy.name} (${record.createdBy.email})`,
          `Établissement : ${etablissement}`,
          `Motif : ${motif}`,
          `Classes / matière : ${classesOuMatiere}`,
          `Nombre : ${nb}`,
          attachments.length === 1
            ? `Document à imprimer : joint à cet e-mail.`
            : attachments.length > 1
              ? `${attachments.length} documents à imprimer : joints à cet e-mail.`
              : "",
          ``,
          `Traiter : ${link}`,
        ]
          .filter(Boolean)
          .join("\n"),
        ...(attachments.length > 0 ? { attachments } : {}),
      });
    }
  } catch (err) {
    console.warn("[brain-ai] photocopies mail failed", err);
  }

  return {
    ok: true,
    data: {
      id: record.id,
      followUrl: "/photocopies-couleur",
      ctas: [{ label: "Suivre la demande", href: "/photocopies-couleur" }],
    },
    summaryFr:
      `Demande photocopies créée (${record.id}) — ${nb} ex. pour ${classesOuMatiere}` +
      (docs.length === 0
        ? "."
        : docs.length === 1
          ? ` avec PDF « ${docs[0].fileName} ».`
          : ` avec ${docs.length} PDF.`),
  };
}
