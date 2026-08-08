import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";
import { auth } from "@clerk/nextjs/server";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson, getObjectBytes } from "@/app/lib/s3-storage";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

const INDEX_KEY = "photocopies-couleur/index.json";

/** Réception des demandes acceptées ; surclassable par PHOTOCOPIES_COULEUR_OPS_EMAIL. */
const DEFAULT_PHOTOCOPIES_OPS_EMAIL = "carine.perier@ac-normandie.fr";

export type PhotoCopieEtablissement = "École" | "Collège" | "Lycée";

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
  decidedBy?: { userId: string; name: string };
  decidedAt?: string;
  directionNote?: string;
};

const norm = (s: string) =>
  String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");

function rolesOfUser(roleRaw: unknown): string[] {
  return Array.isArray(roleRaw) ? (roleRaw as string[]) : roleRaw ? [String(roleRaw)] : [];
}

function getRoleFlags(roles: string[]) {
  const n = roles.map(norm);
  return {
    isDirectionEcole: n.some((r) => r.includes("direction") && r.includes("ecole")),
    isDirectionCollege: n.some((r) => r.includes("direction") && r.includes("college")),
    isDirectionLycee: n.some((r) => r.includes("direction") && r.includes("lycee")),
    isAdministratif: n.some((r) => r.includes("administratif")),
    isProfesseur: n.some((r) => r.includes("professeur")),
    isEducation: n.some((r) => r.includes("education") || r === "cpe"),
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

async function resolveDirectorMail( etab: PhotoCopieEtablissement) {
  const bundle = await loadAppConfig();
  const est = getEstablishmentByLabel(bundle, etab);
  if (est) {
    return { name: est.directorName || est.label, email: est.directorEmail || "", label: est.label };
  }
  return { name: bundle.identity.name, email: "", label: etab };
}

async function getMailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

async function getIndex(): Promise<PhotoCopieRecord[]> {
  const hit = await getJson<PhotoCopieRecord[]>( INDEX_KEY);
  return hit?.data ?? [];
}

async function saveIndex( rows: PhotoCopieRecord[]) {
  await putJson(INDEX_KEY, rows);
}

function isValidDocumentKey(key: string): boolean {
  return key.startsWith("photocopies-couleur/uploads/") && !key.includes("..");
}

async function loadDocumentAttachment(record: PhotoCopieRecord) {
  if (!record.documentKey || !record.documentFileName) return null;
  const bytes = await getObjectBytes(record.documentKey);
  if (!bytes?.length) return null;
  return {
    filename: record.documentFileName,
    content: bytes,
    contentType: record.documentContentType || "application/pdf",
  };
}

function isValidEtab(v: string): v is PhotoCopieEtablissement {
  return v === "École" || v === "Collège" || v === "Lycée";
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesOfUser(user?.publicMetadata?.role);

  if (!canCreateDemand(roles)) {
    const f = getRoleFlags(roles);
    if (!f.isDirectionEcole && !f.isDirectionCollege && !f.isDirectionLycee) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }
  }

  try {
    const all = await getIndex();
    const filtered = all.filter((r) => canViewDemand(r, userId, roles));
    filtered.sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
    return NextResponse.json({ items: filtered });
  } catch (e) {
    console.error("[photocopies-couleur] GET", e);
    return NextResponse.json({ error: "Impossible de charger les demandes." }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesOfUser(user?.publicMetadata?.role);

  if (!canCreateDemand(roles)) {
    return NextResponse.json({ error: "Seuls les enseignants, l'équipe vie scolaire et l'administratif peuvent créer une demande." }, { status: 403 });
  }

  const email = user?.primaryEmailAddress?.emailAddress?.trim() || "";
  if (!email) {
    return NextResponse.json({ error: "Votre compte doit avoir une adresse e-mail pour suivre les réponses." }, { status: 400 });
  }

  let body: Record<string, unknown>;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const etablissement = String(body.etablissement || "").trim();
  if (!isValidEtab(etablissement)) {
    return NextResponse.json({ error: "Établissement invalide (École, Collège ou Lycée)." }, { status: 400 });
  }

  const motif = String(body.motif || "").trim();
  const classeField = String(body.classesOuMatiere ?? "").trim();

  const nb = Number(body.nombrePhotocopies);
  if (!Number.isFinite(nb) || nb < 1 || nb > 1_000_000) {
    return NextResponse.json({ error: "Nombre de photocopies invalide (entier positif)." }, { status: 400 });
  }

  if (!motif || motif.length > 8000) {
    return NextResponse.json({ error: "Le motif est requis." }, { status: 400 });
  }

  if (!classeField || classeField.length > 4000) {
    return NextResponse.json({ error: "Le champ classes / matière est requis." }, { status: 400 });
  }

  const documentKey = String(body.documentKey || "").trim();
  const documentFileName = String(body.documentFileName || "").trim();
  const documentContentType = String(body.documentContentType || "application/pdf").trim();
  if (documentKey && !isValidDocumentKey(documentKey)) {
    return NextResponse.json({ error: "Document joint invalide." }, { status: 400 });
  }
  if (documentKey && !documentFileName) {
    return NextResponse.json({ error: "Nom du fichier requis." }, { status: 400 });
  }

  const record: PhotoCopieRecord = {
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    status: "EN_ATTENTE",
    createdBy: {
      userId,
      name: user?.fullName || user?.firstName || email,
      email,
    },
    etablissement,
    motif,
    classesOuMatiere: classeField,
    nombrePhotocopies: nb,
    ...(documentKey
      ? {
          documentKey,
          documentFileName,
          documentContentType: documentContentType || "application/pdf",
        }
      : {}),
  };

  try {
    const all = await getIndex();
    all.push(record);
    await saveIndex( all);

    const dir = await resolveDirectorMail( etablissement);
    const mail = await getMailer();
    const docAttachment = await loadDocumentAttachment(record);
    const photocopiesLink = await tenantAbsolutePath("/photocopies-couleur");
    if (mail) {
      const { smtp, transporter } = mail;
      try {
        await transporter.sendMail({
          from: `"Demandes photocopies" <${smtp.user}>`,
          to: dir.email,
          subject: `Photocopies couleur — nouvelle demande (${etablissement})`,
          text: [
            `Bonjour ${dir.name},`,
            ``,
            `Une nouvelle demande de photocopies couleur a été déposée sur l’intranet.`,
            ``,
            `Demandeur : ${record.createdBy.name} (${record.createdBy.email})`,
            `Établissement : ${etablissement}`,
            `Motif : ${motif}`,
            `Classes / matière : ${classeField}`,
            `Nombre de photocopies : ${nb}`,
            docAttachment ? `Document à imprimer : joint à cet e-mail.` : "",
            ``,
            `Traiter la demande : ${photocopiesLink}`,
            ``,
            `Cordialement,`,
            `Plateforme La Providence Nicolas Barré`,
          ]
            .filter(Boolean)
            .join("\n"),
          ...(docAttachment ? { attachments: [docAttachment] } : {}),
        });
      } catch (mailErr) {
        console.error("[photocopies-couleur] mail direction:", mailErr);
      }
    } else {
      console.warn("[photocopies-couleur] SMTP non configuré — pas d’email direction.");
    }

    return NextResponse.json({ success: true, id: record.id });
  } catch (e) {
    console.error("[photocopies-couleur] POST", e);
    return NextResponse.json({ error: "Enregistrement impossible." }, { status: 500 });
  }
}

export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesOfUser(user?.publicMetadata?.role);

  let body: { id?: string; status?: string; directionNote?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const id = String(body?.id || "").trim();
  const statusRaw = String(body?.status || "").trim().toUpperCase();
  const directionNote = String(body?.directionNote || "").trim();

  if (!id || !["ACCEPTEE", "REFUSEE"].includes(statusRaw)) {
    return NextResponse.json({ error: "Paramètres invalides." }, { status: 400 });
  }

  try {
    const all = await getIndex();
    const idx = all.findIndex((r) => r.id === id);
    if (idx < 0) return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });

    const current = all[idx];
    if (!canManageDemand(current, roles)) {
      return NextResponse.json({ error: "Décision réservée à la direction concernée." }, { status: 403 });
    }
    if (current.status !== "EN_ATTENTE") {
      return NextResponse.json({ error: "Cette demande a déjà été traitée." }, { status: 400 });
    }

    const updated: PhotoCopieRecord = {
      ...current,
      status: statusRaw as "ACCEPTEE" | "REFUSEE",
      updatedAt: new Date().toISOString(),
      decidedBy: { userId, name: user?.fullName || user?.firstName || "Direction" },
      decidedAt: new Date().toISOString(),
      directionNote: directionNote || undefined,
    };

    all[idx] = updated;
    await saveIndex( all);

    const creatorEmail = updated.createdBy.email;
    const base = await tenantAbsolutePath("/photocopies-couleur");

    const mail = await getMailer();
    if (mail) {
      const { smtp, transporter } = mail;
      try {
        if (creatorEmail) {
          await transporter.sendMail({
            from: `"Demandes photocopies" <${smtp.user}>`,
            to: creatorEmail,
            subject:
              updated.status === "ACCEPTEE"
                ? "Votre demande de photocopies couleur a été acceptée"
                : "Votre demande de photocopies couleur a été refusée",
            text:
              updated.status === "ACCEPTEE"
                ? [
                    `Bonjour ${updated.createdBy.name},`,
                    ``,
                    `Votre demande de photocopies couleur a été acceptée par la direction (${updated.etablissement}).`,
                    `Nombre de photocopies : ${updated.nombrePhotocopies}`,
                    `Motif : ${updated.motif}`,
                    `Classes / matière : ${updated.classesOuMatiere}`,
                    directionNote ? `Message de la direction : ${directionNote}` : "",
                    ``,
                    `Détail sur l’intranet : ${base}`,
                    ``,
                    `Cordialement,`,
                    `La Providence Nicolas Barré`,
                  ]
                    .filter(Boolean)
                    .join("\n")
                : [
                    `Bonjour ${updated.createdBy.name},`,
                    ``,
                    `Votre demande de photocopies couleur n’a pas été acceptée.`,
                    `Établissement : ${updated.etablissement}`,
                    directionNote ? `Motif précisé par la direction : ${directionNote}` : "",
                    ``,
                    `Détail : ${base}`,
                    ``,
                    `Cordialement,`,
                    `La Providence Nicolas Barré`,
                  ]
                    .filter(Boolean)
                    .join("\n"),
          });
        }
      } catch (e) {
        console.error("[photocopies-couleur] mail demandeur:", e);
      }

      const nCfg = (await loadAppConfig()).notifications;
      const opsMail =
        process.env.PHOTOCOPIES_COULEUR_OPS_EMAIL?.trim() || nCfg.photocopiesOps || DEFAULT_PHOTOCOPIES_OPS_EMAIL;
      if (updated.status === "ACCEPTEE") {
        const opsAttachment = await loadDocumentAttachment(updated);
        try {
          await transporter.sendMail({
            from: `"Demandes photocopies" <${smtp.user}>`,
            to: opsMail,
            subject: `[À traiter] Photocopies couleur acceptées — ${updated.etablissement}`,
            text: [
              `Bonjour,`,
              ``,
              `Une demande de photocopies couleur a été ACCEPTÉE par la direction : vous pouvez procéder à l'impression.`,
              ``,
              `Demandeur : ${updated.createdBy.name} (${updated.createdBy.email})`,
              `Établissement : ${updated.etablissement}`,
              `Décision par : ${updated.decidedBy?.name}`,
              `Nombre : ${updated.nombrePhotocopies}`,
              `Motif : ${updated.motif}`,
              `Classes / matière : ${updated.classesOuMatiere}`,
              directionNote ? `Note direction : ${directionNote}` : "",
              opsAttachment
                ? `Le document à imprimer est joint à cet e-mail (${updated.documentFileName}).`
                : `Aucun PDF joint : voir l'intranet ou contacter le demandeur.`,
              ``,
              `Historique intranet : ${base}`,
              ``,
              `Cordialement,`,
              `Plateforme La Providence Nicolas Barré`,
            ]
              .filter(Boolean)
              .join("\n"),
            ...(opsAttachment ? { attachments: [opsAttachment] } : {}),
          });
        } catch (e) {
          console.error("[photocopies-couleur] mail ops:", e);
        }
      }
    }

    return NextResponse.json({ success: true });
  } catch (e) {
    console.error("[photocopies-couleur] PATCH", e);
    return NextResponse.json({ error: "Mise à jour impossible." }, { status: 500 });
  }
}
