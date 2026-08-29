import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import {
  findPersonnelByEmail,
  findPersonnelByExternalId,
  getAllPersonnelRecords,
  savePersonnelRecord,
} from "@/app/lib/personnel-storage";
import {
  canManagePersonnel,
  canViewRecord,
  inferCategoryFromRoles,
  normalizePersonnelRecord,
  sanitizeRecordForViewer,
  uid,
  type PersonnelRecord,
} from "@/app/lib/personnel-types";
import { defaultPersonnelProfile, normalizePersonnelProfile } from "@/app/lib/personnel-profile";
import { readMetaRhByFolderName, readRhPersonnelIndex, writeMetaRh } from "@/app/lib/rh/meta-storage";
import { resolveRhIndexEntryForUser } from "@/app/lib/rh/rh-personnel-match";
import {
  canValidateRhEspace,
  defaultPersonnelRhSpace,
  isRhIdentityComplete,
  resolveRhEspacePhase,
  type RhEspacePhase,
} from "@/app/lib/rh/rh-space-status";
import { canAccessRhPersonalEspace } from "@/app/lib/rh/rh-hub-access";
import type { MetaRhDocument } from "@/app/lib/rh/types";

async function loadUserRhContext(userId: string, email: string) {
  let postgresRecord =
    (userId ? await findPersonnelByExternalId(userId) : null) ||
    (email ? await findPersonnelByEmail(email) : null);

  let meta: MetaRhDocument | null = null;
  let folderName: string | null = null;
  let oneDriveLinked = false;

  const indexHit = await readRhPersonnelIndex();
  if (indexHit.ok) {
    oneDriveLinked = true;
    const entry = resolveRhIndexEntryForUser(indexHit.index.entries, userId, email);
    if (entry) {
      folderName = entry.folderName;
      const metaHit = await readMetaRhByFolderName(entry.folderName);
      if (metaHit.ok) meta = metaHit.meta;
    }
  }

  return { postgresRecord, meta, folderName, oneDriveLinked };
}

function espacePayload(
  roles: string[],
  userId: string,
  email: string,
  ctx: Awaited<ReturnType<typeof loadUserRhContext>>,
) {
  const phase = resolveRhEspacePhase({
    roles,
    meta: ctx.meta,
    postgresRecord: ctx.postgresRecord,
  });

  const record = ctx.postgresRecord
    ? sanitizeRecordForViewer(ctx.postgresRecord, roles, userId, email)
    : null;

  const identity = ctx.meta?.identity;
  const profile = record?.profile;

  return {
    phase,
    oneDriveLinked: ctx.oneDriveLinked,
    folderName: ctx.folderName,
    record,
    metaSummary: ctx.meta
      ? {
          accountStatus: ctx.meta.accountStatus,
          onboardingStatus: ctx.meta.onboarding?.status ?? null,
          birthDate: identity?.birthDate ?? null,
          birthPlace: identity?.birthPlace ?? null,
          displayName: `${identity?.firstName ?? ""} ${identity?.lastName ?? ""}`.trim(),
        }
      : null,
    identityComplete: isRhIdentityComplete({ profile, metaIdentity: identity }),
    submittedAt:
      ctx.postgresRecord?.rhSpace?.submittedAt ?? ctx.meta?.onboarding?.submittedAt ?? null,
    validationNote: ctx.postgresRecord?.rhSpace?.validationNote ?? null,
  };
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress?.trim() ?? "";

  if (!canAccessRhPersonalEspace(roles)) {
    return NextResponse.json({ error: "Accès RH non autorisé." }, { status: 403 });
  }

  try {
    const ctx = await loadUserRhContext(userId, email);
    return NextResponse.json(espacePayload(roles, userId, email, ctx));
  } catch (e) {
    console.error("[rh/espace] GET", e);
    return NextResponse.json({ error: "Impossible de charger votre espace RH." }, { status: 500 });
  }
}

export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress?.trim() ?? "";
  const firstName = user?.firstName?.trim() || "Collaborateur";
  const lastName = user?.lastName?.trim() || "";

  let body: Record<string, unknown>;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const action = String(body.action || "").trim();

  try {
    const ctx = await loadUserRhContext(userId, email);

    if (action === "validate" && canValidateRhEspace(roles)) {
      const targetId = String(body.personnelId || "").trim();
      if (!targetId) {
        return NextResponse.json({ error: "Identifiant dossier manquant." }, { status: 400 });
      }
      const all = await getAllPersonnelRecords();
      const target = all.find((r) => r.id === targetId);
      if (!target) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });
      if (target.rhSpace?.status !== "pending_validation") {
        return NextResponse.json({ error: "Ce dossier n'est pas en attente de validation." }, { status: 400 });
      }
      const note = String(body.validationNote || "").trim();
      target.rhSpace = {
        status: "active",
        submittedAt: target.rhSpace.submittedAt ?? null,
        validatedAt: new Date().toISOString(),
        validatedBy: user?.fullName || userId,
        validationNote: note || null,
      };
      await savePersonnelRecord(target);
      return NextResponse.json({ success: true });
    }

    if (action === "reject" && canValidateRhEspace(roles)) {
      const targetId = String(body.personnelId || "").trim();
      const all = await getAllPersonnelRecords();
      const target = all.find((r) => r.id === targetId);
      if (!target) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });
      const note = String(body.validationNote || "").trim();
      target.rhSpace = {
        status: "onboarding",
        submittedAt: null,
        validatedAt: null,
        validatedBy: user?.fullName || userId,
        validationNote: note || "Complétez votre dossier puis resoumettez.",
      };
      await savePersonnelRecord(target);
      return NextResponse.json({ success: true });
    }

    let record = ctx.postgresRecord;

    if (action === "init" && !record) {
      const category = inferCategoryFromRoles(roles) ?? "administratif";
      const now = new Date().toISOString();
      record = normalizePersonnelRecord({
        id: uid("p"),
        externalUserId: userId,
        email: email || `${userId}@local`,
        firstName,
        lastName,
        displayName: `${firstName} ${lastName}`.trim(),
        category,
        jobTitle: hasRole(roles, "professeur") ? "Professeur" : undefined,
        active: true,
        createdAt: now,
        updatedAt: now,
        documents: [],
        formations: [],
        habilitations: [],
        profile: defaultPersonnelProfile(),
        rhSpace: defaultPersonnelRhSpace(),
      });
      await savePersonnelRecord(record);
    }

    if (!record) {
      return NextResponse.json({ error: "Aucun dossier RH — initialisez d'abord votre espace." }, { status: 404 });
    }

    if (!canViewRecord(roles, record, userId, email) && !canManagePersonnel(roles)) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }

    if (action === "update-profile") {
      const patch = normalizePersonnelProfile(body.profile);
      record.profile = { ...defaultPersonnelProfile(), ...record.profile, ...patch };
      record.updatedAt = new Date().toISOString();
      if (!record.rhSpace) record.rhSpace = defaultPersonnelRhSpace();
      await savePersonnelRecord(record);

      if (ctx.folderName && ctx.meta) {
        const m = ctx.meta;
        m.identity = {
          ...m.identity,
          firstName: record.firstName || m.identity.firstName,
          lastName: record.lastName || m.identity.lastName,
          birthDate: patch.birthDate ?? m.identity.birthDate,
          birthPlace: patch.birthPlace ?? m.identity.birthPlace,
          phone: patch.phone ?? m.identity.phone,
          phoneMobile: patch.phoneMobile ?? m.identity.phoneMobile,
          socialSecurityNumber: patch.socialSecurityNumber ?? m.identity.socialSecurityNumber,
          address: {
            line1: patch.addressLine1 ?? m.identity.address?.line1,
            line2: patch.addressLine2 ?? m.identity.address?.line2,
            postalCode: patch.postalCode ?? m.identity.address?.postalCode,
            city: patch.city ?? m.identity.address?.city,
            country: patch.country ?? m.identity.address?.country,
          },
        };
        m.updatedAt = new Date().toISOString();
        await writeMetaRh(ctx.folderName, m);
      }

      const refreshed = await loadUserRhContext(userId, email);
      return NextResponse.json(espacePayload(roles, userId, email, refreshed));
    }

    if (action === "submit-onboarding") {
      if (!isRhIdentityComplete({ profile: record.profile, metaIdentity: ctx.meta?.identity })) {
        return NextResponse.json(
          { error: "Complétez votre date de naissance, adresse et téléphone avant de soumettre." },
          { status: 400 },
        );
      }
      record.rhSpace = {
        status: "pending_validation",
        submittedAt: new Date().toISOString(),
        validatedAt: null,
        validatedBy: null,
        validationNote: null,
      };
      record.updatedAt = new Date().toISOString();
      await savePersonnelRecord(record);

      if (ctx.folderName && ctx.meta) {
        const m = ctx.meta;
        m.accountStatus = "pending";
        m.onboarding = {
          id: m.onboarding?.id ?? uid("onb"),
          status: "soumis",
          submittedAt: new Date().toISOString(),
          startDate: m.onboarding?.startDate ?? new Date().toISOString().slice(0, 10),
        };
        m.updatedAt = new Date().toISOString();
        await writeMetaRh(ctx.folderName, m);
      }

      const refreshed = await loadUserRhContext(userId, email);
      return NextResponse.json(espacePayload(roles, userId, email, refreshed));
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    console.error("[rh/espace] PATCH", e);
    return NextResponse.json({ error: "Mise à jour impossible." }, { status: 500 });
  }
}
