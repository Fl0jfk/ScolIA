import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson, getObjectBytes } from "@/app/lib/s3-storage";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  canCreatePhotocopiesDemand,
  canDeclarePhotocopiesOnBehalf,
  canManagePhotocopiesDemand,
  canProcessPhotocopiesOps,
  canViewPhotocopiesDemand,
  getPhotocopiesRoleFlags,
} from "@/app/lib/photocopies-couleur-access";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import {
  isPhotocopiesOpsHandler,
  resolvePhotocopiesOpsEmails,
} from "@/app/lib/photocopies-couleur-ops";
import type { PhotoCopieRecord } from "@/app/lib/photocopies-couleur-types";

const INDEX_KEY = "photocopies-couleur/index.json";

async function resolveDirectorMail(etab: string) {
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
  const hit = await getJson<PhotoCopieRecord[]>(INDEX_KEY);
  return hit?.data ?? [];
}

async function saveIndex(rows: PhotoCopieRecord[]) {
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

function isValidEtab(v: string, establishments: Establishment[]): boolean {
  return Boolean(matchEstablishment(establishments, v));
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress?.trim() || "";
  const bundle = await loadAppConfig();
  const opsEmails = resolvePhotocopiesOpsEmails(bundle.notifications);
  const isOps = isPhotocopiesOpsHandler(email, opsEmails);

  if (!canCreatePhotocopiesDemand(roles) && !isOps) {
    const f = getPhotocopiesRoleFlags(roles);
    if (!f.isDirection) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }
  }

  try {
    const all = await getIndex();
    const filtered = all.filter((r) =>
      canViewPhotocopiesDemand(r, userId, roles, bundle.establishments, { isOpsHandler: isOps }),
    );
    filtered.sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
    return NextResponse.json({ items: filtered, isOpsHandler: isOps });
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
  const roles = rolesFromUserLike(user);

  if (!canCreatePhotocopiesDemand(roles)) {
    return NextResponse.json(
      { error: "Seuls les enseignants, l'équipe vie scolaire et l'administratif peuvent créer une demande." },
      { status: 403 },
    );
  }

  let body: Record<string, unknown>;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const actorName = user?.fullName || user?.firstName || "Utilisateur";
  const actorEmail = user?.primaryEmailAddress?.emailAddress?.trim() || "";

  const onBehalfRaw = body?.onBehalfOf && typeof body.onBehalfOf === "object" ? body.onBehalfOf : null;
  const onBehalfUserId = onBehalfRaw ? String((onBehalfRaw as { userId?: string }).userId || "").trim() : "";

  let subjectUserId = userId;
  let subjectName = actorName;
  let subjectEmail = actorEmail;
  let submittedBy: PhotoCopieRecord["submittedBy"];

  if (onBehalfUserId) {
    if (!canDeclarePhotocopiesOnBehalf(roles)) {
      return NextResponse.json(
        { error: "Seuls l'administratif, la comptabilité et la direction peuvent déposer une demande pour un enseignant." },
        { status: 403 },
      );
    }
    if (onBehalfUserId === userId) {
      return NextResponse.json(
        { error: "Pour vous-même, décochez « Pour une autre personne »." },
        { status: 400 },
      );
    }
    const members = await listDirectoryMembers();
    const subject = members.find((m) => m.externalUserId === onBehalfUserId && !m.pending);
    if (!subject) {
      return NextResponse.json({ error: "Enseignant introuvable dans l'annuaire." }, { status: 404 });
    }
    subjectUserId = subject.externalUserId;
    subjectName =
      subject.displayName?.trim() ||
      `${subject.firstName ?? ""} ${subject.lastName ?? ""}`.trim() ||
      subject.email;
    subjectEmail = subject.email || "";
    if (!subjectEmail) {
      return NextResponse.json(
        { error: "L'enseignant choisi n'a pas d'adresse e-mail : impossible de le notifier." },
        { status: 400 },
      );
    }
    submittedBy = {
      userId,
      name: actorName,
      email: actorEmail,
      roles,
    };
  } else if (!subjectEmail) {
    return NextResponse.json(
      { error: "Votre compte doit avoir une adresse e-mail pour suivre les réponses." },
      { status: 400 },
    );
  }

  const etablissement = String(body.etablissement || "").trim();
  const bundle = await loadAppConfig();
  if (!isValidEtab(etablissement, bundle.establishments)) {
    return NextResponse.json({ error: "Établissement invalide. Choisissez un site configuré." }, { status: 400 });
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
      userId: subjectUserId,
      name: subjectName,
      email: subjectEmail,
    },
    ...(submittedBy ? { submittedBy } : {}),
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
    await saveIndex(all);

    const dir = await resolveDirectorMail(etablissement);
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
            `Une nouvelle demande de photocopies couleur a été déposée sur l'intranet.`,
            ``,
            `Demandeur : ${record.createdBy.name} (${record.createdBy.email})`,
            submittedBy
              ? `Déposée par : ${submittedBy.name} (${submittedBy.email}) pour le compte de l'enseignant.`
              : "",
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
      console.warn("[photocopies-couleur] SMTP non configuré — pas d'email direction.");
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
  const roles = rolesFromUserLike(user);
  const actorEmail = user?.primaryEmailAddress?.emailAddress?.trim() || "";
  const actorName = user?.fullName || user?.firstName || "Utilisateur";

  let body: { id?: string; status?: string; directionNote?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const id = String(body?.id || "").trim();
  const statusRaw = String(body?.status || "").trim().toUpperCase();
  const directionNote = String(body?.directionNote || "").trim();

  if (!id || !["ACCEPTEE", "REFUSEE", "PRETE"].includes(statusRaw)) {
    return NextResponse.json({ error: "Paramètres invalides." }, { status: 400 });
  }

  try {
    const all = await getIndex();
    const idx = all.findIndex((r) => r.id === id);
    if (idx < 0) return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });

    const current = all[idx];
    const bundle = await loadAppConfig();
    const opsEmails = resolvePhotocopiesOpsEmails(bundle.notifications);
    const isOps = isPhotocopiesOpsHandler(actorEmail, opsEmails);
    const base = await tenantAbsolutePath("/photocopies-couleur");

    // —— Ops : marquer imprimée (ACCEPTEE → PRETE) ——
    if (statusRaw === "PRETE") {
      if (!canProcessPhotocopiesOps(roles, isOps)) {
        return NextResponse.json(
          { error: "Action réservée aux réceptionnaires impressions." },
          { status: 403 },
        );
      }
      if (current.status === "PRETE") {
        return NextResponse.json({ success: true, already: true });
      }
      if (current.status !== "ACCEPTEE") {
        return NextResponse.json(
          { error: "Seules les demandes acceptées par la direction peuvent être marquées imprimées." },
          { status: 400 },
        );
      }

      const updated: PhotoCopieRecord = {
        ...current,
        status: "PRETE",
        updatedAt: new Date().toISOString(),
        readyAt: new Date().toISOString(),
        readyBy: actorName,
      };
      all[idx] = updated;
      await saveIndex(all);

      const mail = await getMailer();
      const creatorEmail = updated.createdBy.email?.trim();
      if (mail && creatorEmail) {
        const { smtp, transporter } = mail;
        try {
          await transporter.sendMail({
            from: `"Demandes photocopies" <${smtp.user}>`,
            to: creatorEmail,
            subject: "Vos photocopies couleur sont prêtes",
            text: [
              `Bonjour ${updated.createdBy.name},`,
              ``,
              `Vos photocopies couleur sont prêtes à être retirées.`,
              ``,
              `Établissement : ${updated.etablissement}`,
              `Nombre : ${updated.nombrePhotocopies}`,
              `Classes / matière : ${updated.classesOuMatiere}`,
              `Marqué prêt par : ${actorName}`,
              ``,
              `Consulter vos demandes : ${base}`,
              ``,
              `Cordialement,`,
              `La Providence Nicolas Barré`,
            ].join("\n"),
            html: `<p>Bonjour ${updated.createdBy.name},</p>
<p><strong>Vos photocopies couleur sont prêtes</strong> à être retirées.</p>
<ul>
<li>Établissement : ${updated.etablissement}</li>
<li>Nombre : ${updated.nombrePhotocopies}</li>
<li>Classes / matière : ${updated.classesOuMatiere}</li>
</ul>
<p><a href="${base}">Voir mes demandes sur l'intranet</a></p>`,
          });
        } catch (e) {
          console.error("[photocopies-couleur] mail prêt demandeur:", e);
        }
      }

      return NextResponse.json({ success: true });
    }

    // —— Direction : accepter / refuser ——
    if (!canManagePhotocopiesDemand(current, roles, bundle.establishments, userId)) {
      return NextResponse.json({ error: "Décision réservée à la direction concernée." }, { status: 403 });
    }
    if (current.status !== "EN_ATTENTE") {
      return NextResponse.json({ error: "Cette demande a déjà été traitée." }, { status: 400 });
    }

    const updated: PhotoCopieRecord = {
      ...current,
      status: statusRaw as "ACCEPTEE" | "REFUSEE",
      updatedAt: new Date().toISOString(),
      decidedBy: { userId, name: actorName },
      decidedAt: new Date().toISOString(),
      directionNote: directionNote || undefined,
    };

    all[idx] = updated;
    await saveIndex(all);

    const creatorEmail = updated.createdBy.email;
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
                    `Elle est en cours d'impression. Vous serez prévenu(e) lorsqu'elles seront prêtes.`,
                    ``,
                    `Détail sur l'intranet : ${base}`,
                    ``,
                    `Cordialement,`,
                    `La Providence Nicolas Barré`,
                  ]
                    .filter(Boolean)
                    .join("\n")
                : [
                    `Bonjour ${updated.createdBy.name},`,
                    ``,
                    `Votre demande de photocopies couleur n'a pas été acceptée.`,
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

      if (updated.status === "ACCEPTEE" && opsEmails.length > 0) {
        const opsAttachment = await loadDocumentAttachment(updated);
        try {
          await transporter.sendMail({
            from: `"Demandes photocopies" <${smtp.user}>`,
            to: opsEmails.join(", "),
            subject: `[À imprimer] Photocopies couleur — ${updated.createdBy.name} (${updated.etablissement})`,
            text: [
              `Bonjour,`,
              ``,
              `Une demande de photocopies couleur a été acceptée : elle est dans votre file d'impression sur l'intranet.`,
              ``,
              `Demandeur : ${updated.createdBy.name} (${updated.createdBy.email})`,
              updated.submittedBy
                ? `Déposée par : ${updated.submittedBy.name} (${updated.submittedBy.email})`
                : "",
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
              `Ouvrir la file d'impression : ${base}`,
              ``,
              `Cordialement,`,
              `Plateforme La Providence Nicolas Barré`,
            ]
              .filter(Boolean)
              .join("\n"),
            html: `<p>Bonjour,</p>
<p>Une demande de photocopies couleur a été <strong>acceptée</strong> : elle est dans votre <strong>file d'impression</strong> sur l'intranet.</p>
<ul>
<li>Demandeur : ${updated.createdBy.name} (${updated.createdBy.email})</li>
${updated.submittedBy ? `<li>Déposée par : ${updated.submittedBy.name}</li>` : ""}
<li>Établissement : ${updated.etablissement}</li>
<li>Nombre : ${updated.nombrePhotocopies}</li>
<li>Classes / matière : ${updated.classesOuMatiere}</li>
</ul>
<p style="margin:1.5rem 0;">
  <a href="${base}" style="display:inline-block;padding:12px 24px;background:#0f766e;color:#ffffff;text-decoration:none;border-radius:8px;font-weight:700;">
    Ouvrir la file d'impression
  </a>
</p>`,
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
