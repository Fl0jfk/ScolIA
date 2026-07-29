import { getJson, putJson, getObjectBytes } from "@/app/lib/s3-storage";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

const INDEX_KEY = "photocopies-couleur/index.json";

type PhotoCopieEtablissement = "École" | "Collège" | "Lycée";

type PhotoCopieRecord = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE";
  createdBy: { userId: string; name: string; email: string };
  etablissement: PhotoCopieEtablissement;
  motif: string;
  classesOuMatiere: string;
  nombrePhotocopies: number;
  documentKey?: string;
  documentFileName?: string;
  documentContentType?: string;
};

function isValidDocumentKey(key: string): boolean {
  return (
    (key.startsWith("photocopies-couleur/uploads/") || key.startsWith("brain-ai/uploads/")) &&
    !key.includes("..")
  );
}

const norm = (s: string) =>
  String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");

function getRoleFlags(roles: string[]) {
  const n = roles.map(norm);
  return {
    isDirectionEcole: n.some((r) => r.includes("direction") && r.includes("ecole")),
    isDirectionCollege: n.some((r) => r.includes("direction") && r.includes("college")),
    isDirectionLycee: n.some((r) => r.includes("direction") && r.includes("lycee")),
    isAdministratif: n.some((r) => r.includes("administratif")),
    isProfesseur: n.some((r) => r.includes("professeur")),
    isEducation: n.some((r) => r.includes("education")),
  };
}

function canCreateDemand(roles: string[]) {
  const f = getRoleFlags(roles);
  return f.isProfesseur || f.isAdministratif || f.isEducation;
}

function canManageDemand(rec: PhotoCopieRecord, roles: string[]) {
  const f = getRoleFlags(roles);
  if (rec.etablissement === "École") return f.isDirectionEcole;
  if (rec.etablissement === "Collège") return f.isDirectionCollege;
  if (rec.etablissement === "Lycée") return f.isDirectionLycee;
  return false;
}

function canViewDemand(rec: PhotoCopieRecord, userId: string, roles: string[]) {
  if (rec.createdBy.userId === userId) return true;
  return canManageDemand(rec, roles);
}

function isValidEtab(v: string): v is PhotoCopieEtablissement {
  return v === "École" || v === "Collège" || v === "Lycée";
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
  const f = getRoleFlags(ctx.roles);
  if (
    !canCreateDemand(ctx.roles) &&
    !f.isDirectionEcole &&
    !f.isDirectionCollege &&
    !f.isDirectionLycee
  ) {
    return { ok: false, error: "Accès réservé aux photocopies couleur.", code: "MODULE_FORBIDDEN" };
  }

  const statusFilter = typeof args.status === "string" ? args.status.trim().toUpperCase() : "";
  const limit = Math.min(Math.max(Number(args.limit) || 15, 1), 40);
  const all = await getIndex();
  let items = all
    .filter((r) => canViewDemand(r, ctx.userId!, ctx.roles))
    .sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
  if (statusFilter) {
    items = items.filter((r) => r.status === statusFilter);
  }
  const brief = items.slice(0, limit).map((r) => ({
    id: r.id,
    status: r.status,
    etablissement: r.etablissement,
    nombrePhotocopies: r.nombrePhotocopies,
    motif: r.motif.slice(0, 120),
    classesOuMatiere: r.classesOuMatiere,
    hasDocument: Boolean(r.documentKey),
    createdAt: r.createdAt.slice(0, 10),
    mine: r.createdBy.userId === ctx.userId,
  }));

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
  if (!canCreateDemand(ctx.roles)) {
    return {
      ok: false,
      error: "Seuls les enseignants, la vie scolaire et l'administratif peuvent créer une demande.",
      code: "MODULE_FORBIDDEN",
    };
  }
  if (!ctx.email) {
    return { ok: false, error: "Votre compte doit avoir une adresse e-mail." };
  }

  const etablissement = String(args.etablissement || "").trim();
  if (!isValidEtab(etablissement)) {
    return { ok: false, error: "Établissement invalide (École, Collège ou Lycée)." };
  }
  const motif = String(args.motif || "").trim();
  const classesOuMatiere = String(args.classesOuMatiere || "").trim();
  const nb = Number(args.nombrePhotocopies);
  const documentKey = String(args.documentKey || "").trim();
  const documentFileName = String(args.documentFileName || "").trim();
  const documentContentType = String(args.documentContentType || "application/pdf").trim();

  if (!motif) return { ok: false, error: "Le motif est requis." };
  if (!classesOuMatiere) return { ok: false, error: "Classes / matière requis." };
  if (!Number.isFinite(nb) || nb < 1 || nb > 1_000_000) {
    return { ok: false, error: "Nombre de photocopies invalide." };
  }
  if (documentKey && !isValidDocumentKey(documentKey)) {
    return { ok: false, error: "Document joint invalide." };
  }
  if (documentKey && !documentFileName) {
    return { ok: false, error: "Nom du fichier PDF requis avec la pièce jointe." };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_photocopie_demand",
      args: {
        etablissement,
        motif,
        classesOuMatiere,
        nombrePhotocopies: nb,
        ...(documentKey
          ? { documentKey, documentFileName, documentContentType }
          : {}),
      },
      summaryFr:
        `Récapitulatif — ${nb} photocopie(s) couleur\n` +
        `• Établissement : ${etablissement}\n` +
        `• Classes / matière : ${classesOuMatiere}\n` +
        `• Motif : ${motif.slice(0, 160)}${motif.length > 160 ? "…" : ""}\n` +
        (documentFileName
          ? `• PDF joint : ${documentFileName}`
          : `• PDF : aucun (vous pouvez encore en joindre un via le trombone avant de confirmer)`),
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
    ...(documentKey
      ? {
          documentKey,
          documentFileName,
          documentContentType: documentContentType || "application/pdf",
        }
      : {}),
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
      let attachments: Array<{ filename: string; content: Buffer; contentType: string }> | undefined;
      if (record.documentKey && record.documentFileName) {
        const bytes = await getObjectBytes(record.documentKey);
        if (bytes?.length) {
          attachments = [
            {
              filename: record.documentFileName,
              content: bytes,
              contentType: record.documentContentType || "application/pdf",
            },
          ];
        }
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
          attachments?.length ? `Document à imprimer : joint à cet e-mail.` : "",
          ``,
          `Traiter : ${link}`,
        ]
          .filter(Boolean)
          .join("\n"),
        ...(attachments ? { attachments } : {}),
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
      (documentFileName ? ` avec PDF « ${documentFileName} ».` : "."),
  };
}
