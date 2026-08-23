import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth, requireModule } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";
import {
  ensureDirectoryUserForPersonnel,
  findDirectoryMemberByEmail,
  getDirectoryMemberById,
  listRhDirectoryCandidates,
  suggestPersonnelCategoryFromRoles,
} from "@/app/lib/personnel-directory";
import { normalizePersonnelProfile } from "@/app/lib/personnel-profile";
import {
  getPersonnelIndex,
  getPersonnelRecord,
  savePersonnelRecord,
  getSharedPersonnelDocuments,
  saveSharedPersonnelDocuments,
} from "@/app/lib/personnel-storage";
import {
  canAccessPersonnelModule,
  canManagePersonnel,
  canViewPersonnelDashboard,
  defaultMedecineTravail,
  defaultOnboarding,
  normalizePersonnelRecord,
  sanitizeRecordForViewer,
  uid,
  type PersonnelCategory,
  type PersonnelRecord,
  type SharedPersonnelDocument,
} from "@/app/lib/personnel-types";


async function assertNotAlreadyInRh(email: string, externalUserId?: string | null) {
  const index = await getPersonnelIndex();
  const normalized = email.trim().toLowerCase();
  if (index.some((e) => e.email.trim().toLowerCase() === normalized)) {
    throw new Error("Un dossier RH existe déjà pour cet email.");
  }
  if (externalUserId && index.some((e) => e.externalUserId === externalUserId)) {
    throw new Error("Ce compte est déjà lié à un dossier RH.");
  }
}

export async function GET(req: Request) {
  const gate = await requireModule("rh");
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessPersonnelModule(roles)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  const url = new URL(req.url);
  const sharedOnly = url.searchParams.get("shared") === "true";

  try {
    if (sharedOnly) {
      const sharedDocs = await getSharedPersonnelDocuments();
      return NextResponse.json({ sharedDocs });
    }

    const index = await getPersonnelIndex();
    if (!canViewPersonnelDashboard(roles)) {
      return NextResponse.json({ index: [], canManage: false });
    }

    await writeDataAccessAudit({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      resourceType: "personnel",
      action: "list",
      req,
      metadata: { count: index.length },
    });

    return NextResponse.json({
      index: index.filter((e) => e.active !== false),
      canManage: canManagePersonnel(roles),
      sharedDocs: await getSharedPersonnelDocuments(),
    });
  } catch (e) {
    console.error("[personnel] GET", e);
    return NextResponse.json({ error: "Erreur chargement." }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canManagePersonnel(roles)) {
    return NextResponse.json({ error: "Création réservée à la RH / comptabilité." }, { status: 403 });
  }

  try {
    const body = await req.json();
    const action = String(body?.action || "create");

    if (action === "shared-doc") {
      const doc = body.document as SharedPersonnelDocument;
      if (!doc?.name || !doc?.fileUrl) {
        return NextResponse.json({ error: "Document invalide." }, { status: 400 });
      }
      const docs = await getSharedPersonnelDocuments();
      const entry: SharedPersonnelDocument = {
        id: doc.id || uid("sd"),
        name: doc.name,
        fileUrl: doc.fileUrl,
        uploadedAt: new Date().toISOString(),
        uploadedBy: user?.fullName || user?.id || "RH",
      };
      docs.push(entry);
      await saveSharedPersonnelDocuments(docs);
      return NextResponse.json({ success: true, document: entry });
    }

    const mode = String(body.mode || "create");
    const jobTitle = String(body.jobTitle || "").trim();
    const hireDate = String(body.hireDate || "").trim() || null;
    const withOnboarding = body.withOnboarding !== false;
    const now = new Date().toISOString();

    let firstName = "";
    let lastName = "";
    let email = "";
    let category = String(body.category || "administratif") as PersonnelCategory;
    let externalUserId: string | null = body.externalUserId ? String(body.externalUserId) : null;
    let directoryInfo: { mode?: string; pending?: boolean } | null = null;

    if (mode === "link-directory") {
      const directoryUserId = body.externalUserId ? String(body.externalUserId).trim() : "";
      const lookupEmail = String(body.email || "").trim().toLowerCase();

      let member = directoryUserId ? await getDirectoryMemberById(directoryUserId) : null;
      if (!member && lookupEmail) {
        member = await findDirectoryMemberByEmail(lookupEmail);
      }

      if (!member) {
        return NextResponse.json({ error: "Utilisateur introuvable." }, { status: 404 });
      }
      firstName = String(member.firstName || "").trim();
      lastName = String(member.lastName || "").trim();
      email = member.email.trim().toLowerCase();
      externalUserId = member.externalUserId || null;
      category = (body.category as PersonnelCategory) || suggestPersonnelCategoryFromRoles(member.roles);

      if (body.firstName) firstName = String(body.firstName).trim();
      if (body.lastName) lastName = String(body.lastName).trim();

      if (!firstName && !lastName) {
        const local = email.split("@")[0]?.split(/[._-]/) || [];
        firstName = local[0] || "Collaborateur";
        lastName = local.slice(1).join(" ") || "RH";
      }

      if (body.category) category = body.category as PersonnelCategory;

      await assertNotAlreadyInRh(email, externalUserId);
    } else {
      firstName = String(body.firstName || "").trim();
      lastName = String(body.lastName || "").trim();
      email = String(body.email || "").trim().toLowerCase();

      if (!firstName || !lastName || !email) {
        return NextResponse.json({ error: "Nom, prénom et email requis." }, { status: 400 });
      }

      await assertNotAlreadyInRh(email, externalUserId);

      const existingDirectory = await findDirectoryMemberByEmail(email);
      if (existingDirectory) {
        externalUserId = existingDirectory.externalUserId || null;
        directoryInfo = {
          mode: existingDirectory.externalUserId ? "directory_linked" : "directory_invitation_pending",
          pending: existingDirectory.pending,
        };
        if (existingDirectory.externalUserId) {
          const synced = await ensureDirectoryUserForPersonnel({ email, firstName, lastName, category });
          externalUserId = synced.externalUserId ?? externalUserId;
        }
      } else if (body.createDirectoryUserUser !== false) {
        const directoryUser = await ensureDirectoryUserForPersonnel({ email, firstName, lastName, category });
        externalUserId = directoryUser.externalUserId;
        directoryInfo = { mode: directoryUser.mode, pending: directoryUser.pending };
      }
    }

    const profile =
      body.profile && typeof body.profile === "object"
        ? normalizePersonnelProfile(body.profile)
        : undefined;

    const record: PersonnelRecord = normalizePersonnelRecord({
      id: uid("p"),
      externalUserId,
      email,
      firstName,
      lastName,
      displayName: `${firstName} ${lastName}`.trim(),
      category,
      jobTitle: jobTitle || undefined,
      hireDate,
      active: true,
      createdAt: now,
      updatedAt: now,
      documents: [],
      formations: [],
      habilitations: [],
      medecineTravail: defaultMedecineTravail(),
      entretiens: [],
      onboarding: withOnboarding ? defaultOnboarding(hireDate || now.slice(0, 10)) : null,
      profile,
    });

    const saved = await savePersonnelRecord(record);
    return NextResponse.json({ success: true, record: saved, directory: directoryInfo });
  } catch (e) {
    console.error("[personnel] POST", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement.";
    const status = msg.includes("existe déjà") || msg.includes("déjà lié") ? 409 : 500;
    return NextResponse.json({ error: msg }, { status });
  }
}

export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress || "";

  try {
    const body = await req.json();

    if (body.action === "delete-shared-doc") {
      if (!canManagePersonnel(roles)) {
        return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
      }
      const docId = String(body.docId || "");
      const docs = (await getSharedPersonnelDocuments()).filter((d) => d.id !== docId);
      await saveSharedPersonnelDocuments(docs);
      return NextResponse.json({ success: true });
    }

    const id = String(body.id || "").trim();
    if (!id) return NextResponse.json({ error: "ID manquant." }, { status: 400 });

    const existing = await getPersonnelRecord(id);
    if (!existing) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

    const isRh = canManagePersonnel(roles);
    const isSelf =
      (existing.externalUserId && existing.externalUserId === user?.id) ||
      existing.email.trim().toLowerCase() === email.trim().toLowerCase();

    if (!isRh && !isSelf) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }

    const updated: PersonnelRecord = {
      ...existing,
      ...body.patch,
      id: existing.id,
      updatedAt: new Date().toISOString(),
    };

    if (!isRh) {
      // Personnel : uniquement lecture — pas de patch global
      return NextResponse.json({ error: "Modification réservée à la RH." }, { status: 403 });
    }

    const saved = await savePersonnelRecord(normalizePersonnelRecord(updated));
    return NextResponse.json({
      record: sanitizeRecordForViewer(saved, roles, user?.id, email),
    });
  } catch (e) {
    console.error("[personnel] PATCH", e);
    return NextResponse.json({ error: "Erreur mise à jour." }, { status: 500 });
  }
}
