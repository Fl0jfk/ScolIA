import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { getPersonnelRecord, savePersonnelRecord } from "@/app/lib/personnel-storage";
import {
  computeNextEntretienDue,
  normalizeMedecineTravail,
  syncMedecineDerivedFields,
} from "@/app/lib/personnel-rh-cycles";
import { normalizePersonnelProfile } from "@/app/lib/personnel-profile";
import { notifyPersonnelSignatureLink } from "@/app/lib/personnel-notify";
import {
  attachSignatureToken,
  generatePersonnelSignToken,
  savePersonnelSignatureRef,
} from "@/app/lib/personnel-signature";
import {
  canManagePersonnel,
  canViewRecord,
  defaultOffboarding,
  sanitizeRecordForViewer,
  normalizePersonnelRecord,
  uid,
  type DocumentVisibility,
  type PersonnelDocCategory,
  type PersonnelDocument,
  type PersonnelMedecineVisit,
} from "@/app/lib/personnel-types";


export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress || "";

  try {
    const record = await getPersonnelRecord(id);
    if (!record) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

    if (!canViewRecord(roles, record, user?.id, email)) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }

    return NextResponse.json({
      record: sanitizeRecordForViewer(record, roles, user?.id, email),
      canManage: canManagePersonnel(roles),
    });
  } catch (e) {
    console.error("[personnel/[id]] GET", e);
    return NextResponse.json({ error: "Erreur chargement." }, { status: 500 });
  }
}

export async function PATCH(req: Request, ctx: { params: Promise<{ id: string }> }) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  if (!canManagePersonnel(roles)) {
    return NextResponse.json({ error: "Modification réservée à la RH." }, { status: 403 });
  }

  try {
    const body = await req.json();
    const record = await getPersonnelRecord(id);
    if (!record) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

    const action = String(body.action || "update");

    if (action === "add-document") {
      const doc: PersonnelDocument = {
        id: uid("doc"),
        name: String(body.name || "Document"),
        fileUrl: String(body.fileUrl || ""),
        s3Key: body.s3Key ? String(body.s3Key) : undefined,
        category: (body.category || "autre") as PersonnelDocCategory,
        visibility: (body.visibility || "establishment") as DocumentVisibility,
        uploadedAt: new Date().toISOString(),
        uploadedBy: user?.fullName || user?.id || "RH",
        expiresAt: body.expiresAt || null,
      };
      if (!doc.fileUrl) return NextResponse.json({ error: "URL fichier manquante." }, { status: 400 });
      record.documents = [...record.documents, doc];
    } else if (action === "remove-document") {
      const docId = String(body.docId || "");
      record.documents = record.documents.filter((d) => d.id !== docId);
    } else if (action === "add-formation") {
      record.formations = [
        ...record.formations,
        {
          id: uid("form"),
          title: String(body.title || "Formation"),
          status: body.status || "planifiee",
          plannedDate: body.plannedDate || null,
          completedDate: null,
          reminderAt: body.reminderAt || body.plannedDate || null,
          notes: body.notes || "",
        },
      ];
    } else if (action === "update-formation") {
      const formId = String(body.formId || "");
      record.formations = record.formations.map((f) =>
        f.id === formId ? { ...f, ...body.patch } : f,
      );
    } else if (action === "add-habilitation") {
      record.habilitations = [
        ...record.habilitations,
        {
          id: uid("hab"),
          label: String(body.label || "Habilitation"),
          obtainedAt: body.obtainedAt || null,
          expiresAt: String(body.expiresAt || ""),
          documentId: body.documentId || null,
          notes: body.notes || "",
        },
      ];
    } else if (action === "update-habilitation") {
      const habId = String(body.habId || "");
      record.habilitations = record.habilitations.map((h) =>
        h.id === habId ? { ...h, ...body.patch } : h,
      );
    } else if (action === "update-medecine") {
      record.medecineTravail = syncMedecineDerivedFields({
        ...normalizeMedecineTravail(record.medecineTravail),
        ...body.patch,
      });
    } else if (action === "add-medecine-visit") {
      const visitedAt = String(body.visitedAt || "").slice(0, 10);
      if (!visitedAt) return NextResponse.json({ error: "Date de visite requise." }, { status: 400 });
      const visit: PersonnelMedecineVisit = {
        id: uid("medv"),
        visitedAt,
        visitType: String(body.visitType || ""),
        documentId: body.documentId || null,
        notes: String(body.notes || ""),
        createdAt: new Date().toISOString(),
      };
      const med = normalizeMedecineTravail(record.medecineTravail);
      record.medecineTravail = syncMedecineDerivedFields({
        ...med,
        visits: [...(med.visits || []), visit],
        notes: body.notes ? String(body.notes) : med.notes,
      });
    } else if (action === "add-entretien") {
      record.entretiens = [
        ...record.entretiens,
        {
          id: uid("ent"),
          status: body.status || "a_planifier",
          scheduledAt: body.scheduledAt || null,
          completedAt: null,
          reminderAt: body.reminderAt || null,
          nextDueAt: null,
          notes: body.notes || "",
        },
      ];
    } else if (action === "add-entretien-realise") {
      const completedAt = String(body.completedAt || new Date().toISOString().slice(0, 10)).slice(0, 10);
      record.entretiens = [
        ...record.entretiens,
        {
          id: uid("ent"),
          status: "realise",
          scheduledAt: body.scheduledAt || completedAt,
          completedAt,
          reminderAt: null,
          nextDueAt: computeNextEntretienDue(completedAt),
          documentId: body.documentId || null,
          notes: String(body.notes || ""),
        },
      ];
    } else if (action === "update-entretien") {
      const entId = String(body.entId || "");
      record.entretiens = record.entretiens.map((e) => {
        if (e.id !== entId) return e;
        const merged = { ...e, ...body.patch };
        if (merged.status === "realise" && merged.completedAt && !merged.nextDueAt) {
          merged.nextDueAt = computeNextEntretienDue(String(merged.completedAt));
        }
        return merged;
      });
    } else if (action === "update-onboarding") {
      if (!record.onboarding) return NextResponse.json({ error: "Pas d'onboarding." }, { status: 400 });
      record.onboarding = { ...record.onboarding, ...body.patch };
    } else if (action === "toggle-checklist") {
      const itemId = String(body.itemId || "");
      if (!record.onboarding) return NextResponse.json({ error: "Pas d'onboarding." }, { status: 400 });
      record.onboarding.checklist = record.onboarding.checklist.map((c) =>
        c.id === itemId ? { ...c, done: !c.done } : c,
      );
    } else if (action === "sign-onboarding") {
      const sigId = String(body.sigId || "");
      if (!record.onboarding) return NextResponse.json({ error: "Pas d'onboarding." }, { status: 400 });
      record.onboarding.signatures = record.onboarding.signatures.map((s) =>
        s.id === sigId
          ? { ...s, status: "signe", signedAt: new Date().toISOString(), signedBy: user?.fullName || user?.id || "" }
          : s,
      );
      const allSigned = record.onboarding.signatures.every((s) => s.status === "signe");
      if (allSigned) record.onboarding.status = "termine";
      else record.onboarding.status = "signatures";
    } else if (action === "send-signature-link") {
      const sigId = String(body.sigId || "");
      const kind = body.kind === "offboarding" ? "offboarding" : "onboarding";
      const email = String(body.email || "").trim().toLowerCase();
      if (!email) return NextResponse.json({ error: "E-mail requis." }, { status: 400 });

      const flow = kind === "offboarding" ? record.offboarding : record.onboarding;
      if (!flow) return NextResponse.json({ error: "Parcours introuvable." }, { status: 400 });
      const sig = flow.signatures.find((s) => s.id === sigId);
      if (!sig) return NextResponse.json({ error: "Signature introuvable." }, { status: 404 });

      const token = generatePersonnelSignToken();
      if (kind === "offboarding" && record.offboarding) {
        record.offboarding = attachSignatureToken(record.offboarding, sigId, token, email);
      } else if (record.onboarding) {
        record.onboarding = attachSignatureToken(record.onboarding, sigId, token, email);
      }

      await savePersonnelSignatureRef(token, {
        personnelId: record.id,
        kind,
        signatureId: sigId,
        employeeName: record.displayName,
        signatureLabel: sig.label,
        createdAt: new Date().toISOString(),
      });

      const mail = await notifyPersonnelSignatureLink({
        to: email,
        employeeName: record.displayName,
        signatureLabel: sig.label,
        kind,
        token,
      });
      if (!mail.sent) {
        return NextResponse.json({ error: "E-mail non envoyé (SMTP)." }, { status: 503 });
      }
    } else if (action === "start-offboarding") {
      const endDate = String(body.endDate || "").trim();
      if (!endDate) return NextResponse.json({ error: "Date de fin requise." }, { status: 400 });
      record.offboarding = defaultOffboarding(endDate);
      record.active = true;
    } else if (action === "toggle-offboarding-checklist") {
      const itemId = String(body.itemId || "");
      if (!record.offboarding) return NextResponse.json({ error: "Pas d'offboarding." }, { status: 400 });
      record.offboarding.checklist = record.offboarding.checklist.map((c) =>
        c.id === itemId ? { ...c, done: !c.done } : c,
      );
      const allDone = record.offboarding.checklist.every((c) => c.done);
      const allSigned = record.offboarding.signatures.every((s) => s.status === "signe");
      if (allDone && allSigned) {
        record.offboarding.status = "termine";
        record.active = false;
      }
    } else if (action === "sign-offboarding") {
      const sigId = String(body.sigId || "");
      if (!record.offboarding) return NextResponse.json({ error: "Pas d'offboarding." }, { status: 400 });
      record.offboarding.signatures = record.offboarding.signatures.map((s) =>
        s.id === sigId
          ? { ...s, status: "signe", signedAt: new Date().toISOString(), signedBy: user?.fullName || user?.id || "" }
          : s,
      );
      const allSigned = record.offboarding.signatures.every((s) => s.status === "signe");
      const allDone = record.offboarding.checklist.every((c) => c.done);
      if (allSigned && allDone) {
        record.offboarding.status = "termine";
        record.active = false;
      }
    } else if (action === "update-profile") {
      if (body.firstName) record.firstName = String(body.firstName);
      if (body.lastName) record.lastName = String(body.lastName);
      if (body.jobTitle !== undefined) record.jobTitle = String(body.jobTitle || "");
      if (body.category) record.category = body.category;
      if (body.hireDate !== undefined) record.hireDate = body.hireDate;
      if (body.externalUserId !== undefined) record.externalUserId = body.externalUserId;
      if (body.active !== undefined) record.active = !!body.active;
      if (body.email) record.email = String(body.email).trim().toLowerCase();
      if (body.profile && typeof body.profile === "object") {
        record.profile = normalizePersonnelProfile({
          ...(record.profile || {}),
          ...body.profile,
        });
      }
      record.displayName = `${record.firstName} ${record.lastName}`.trim();
    } else {
      return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
    }

    const saved = await savePersonnelRecord(normalizePersonnelRecord(record));
    const email = user?.primaryEmailAddress?.emailAddress || "";
    return NextResponse.json({
      record: sanitizeRecordForViewer(saved, roles, user?.id, email),
    });
  } catch (e) {
    console.error("[personnel/[id]] PATCH", e);
    return NextResponse.json({ error: "Erreur mise à jour." }, { status: 500 });
  }
}
