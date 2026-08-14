import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import {
  ensureClerkUserForPersonnel,
  findClerkMemberByEmail,
  getClerkMemberById,
  listRhClerkCandidates,
  suggestPersonnelCategoryFromClerkRoles,
} from "@/app/lib/personnel-clerk";
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


async function assertNotAlreadyInRh(email: string, clerkUserId?: string | null) {
  const index = await getPersonnelIndex();
  const normalized = email.trim().toLowerCase();
  if (index.some((e) => e.email.trim().toLowerCase() === normalized)) {
    throw new Error("Un dossier RH existe déjà pour cet email.");
  }
  if (clerkUserId && index.some((e) => e.clerkUserId === clerkUserId)) {
    throw new Error("Ce compte Clerk est déjà lié à un dossier RH.");
  }
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessPersonnelModule(roles)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

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
    let clerkUserId: string | null = body.clerkUserId ? String(body.clerkUserId) : null;
    let clerkInfo: { mode?: string; pending?: boolean } | null = null;

    if (mode === "link-clerk") {
      const clerkId = body.clerkUserId ? String(body.clerkUserId).trim() : "";
      const lookupEmail = String(body.email || "").trim().toLowerCase();

      let member = clerkId ? await getClerkMemberById(clerkId) : null;
      if (!member && lookupEmail) {
        member = await findClerkMemberByEmail(lookupEmail);
      }

      if (!member) {
        return NextResponse.json({ error: "Utilisateur Clerk introuvable." }, { status: 404 });
      }
      firstName = String(member.firstName || "").trim();
      lastName = String(member.lastName || "").trim();
      email = member.email.trim().toLowerCase();
      clerkUserId = member.clerkUserId || null;
      category = (body.category as PersonnelCategory) || suggestPersonnelCategoryFromClerkRoles(member.roles);

      if (body.firstName) firstName = String(body.firstName).trim();
      if (body.lastName) lastName = String(body.lastName).trim();

      if (!firstName && !lastName) {
        const local = email.split("@")[0]?.split(/[._-]/) || [];
        firstName = local[0] || "Collaborateur";
        lastName = local.slice(1).join(" ") || "RH";
      }

      if (body.category) category = body.category as PersonnelCategory;

      await assertNotAlreadyInRh(email, clerkUserId);
    } else {
      firstName = String(body.firstName || "").trim();
      lastName = String(body.lastName || "").trim();
      email = String(body.email || "").trim().toLowerCase();

      if (!firstName || !lastName || !email) {
        return NextResponse.json({ error: "Nom, prénom et email requis." }, { status: 400 });
      }

      await assertNotAlreadyInRh(email, clerkUserId);

      const existingClerk = await findClerkMemberByEmail(email);
      if (existingClerk) {
        clerkUserId = existingClerk.clerkUserId || null;
        clerkInfo = {
          mode: existingClerk.clerkUserId ? "clerk_linked" : "clerk_invitation_pending",
          pending: existingClerk.pending,
        };
        if (existingClerk.clerkUserId) {
          const synced = await ensureClerkUserForPersonnel({ email, firstName, lastName, category });
          clerkUserId = synced.clerkUserId ?? clerkUserId;
        }
      } else if (body.createClerkUser !== false) {
        const clerk = await ensureClerkUserForPersonnel({ email, firstName, lastName, category });
        clerkUserId = clerk.clerkUserId;
        clerkInfo = { mode: clerk.mode, pending: clerk.pending };
      }
    }

    const profile =
      body.profile && typeof body.profile === "object"
        ? normalizePersonnelProfile(body.profile)
        : undefined;

    const record: PersonnelRecord = normalizePersonnelRecord({
      id: uid("p"),
      clerkUserId,
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
    return NextResponse.json({ success: true, record: saved, clerk: clerkInfo });
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
      (existing.clerkUserId && existing.clerkUserId === user?.id) ||
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
