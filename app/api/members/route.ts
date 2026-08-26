import { NextResponse } from "next/server";
import { isDatabaseConfigured } from "@/db/index";
import { INTRANET_ROLE_OPTIONS, hasMasterRole, normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { setUserRolesInDb, syncUserAdminFlagsInDb } from "@/app/lib/auth-roles-db";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { findDbUserByExternalId, listMembersFromDb } from "@/app/lib/members-db";
import { membersApiSourceLabel } from "@/app/lib/members-sync";
import { and, eq, ne, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { session, user } from "@/db/schema";

export async function GET(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const tenantScope = await requireTenantId();
  if (!tenantScope.ok) return tenantScope.response;

  try {
    const tenant = await getTenant();
    const etablissementId = isDatabaseConfigured()
      ? await ensureEtablissementFromTenant(tenant)
      : null;

    const users = etablissementId
      ? await listMembersFromDb(etablissementId)
      : await listDirectoryMembers();

    if (etablissementId) {
      await writeDataAccessAudit({
        etablissementId,
        userId: tenantScope.ctx.authUserId,
        resourceType: "members",
        action: "list",
        req,
        metadata: { count: users.length },
      });
    }

    return NextResponse.json({
      users,
      roleOptions: INTRANET_ROLE_OPTIONS,
      source: membersApiSourceLabel(),
    });
  } catch (e) {
    console.error("members GET:", e);
    return NextResponse.json({ error: "Impossible de charger les utilisateurs." }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const email = String(body.email ?? "")
      .trim()
      .toLowerCase();
    const roles = normalizeIntranetRoles(body.intranetRoles ?? body.roles);
    const firstName = String(body.firstName ?? "").trim();
    const lastName = String(body.lastName ?? "").trim();

    if (roles.some((r) => r === "master")) {
      return NextResponse.json({ error: "Le rôle Master ne peut pas être attribué ici." }, { status: 403 });
    }

    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      return NextResponse.json({ error: "Adresse e-mail invalide." }, { status: 400 });
    }
    if (roles.length === 0) {
      return NextResponse.json({ error: "Sélectionnez au moins un rôle." }, { status: 400 });
    }

    if (!isDatabaseConfigured()) {
      return NextResponse.json({ error: "Base de données requise." }, { status: 503 });
    }

    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
    const { getDb } = await import("@/db/index");
    const { user } = await import("@/db/schema");
    const { eq } = await import("drizzle-orm");
    const db = getDb();

    const [existing] = await db.select().from(user).where(eq(user.email, email)).limit(1);
    if (existing) {
      if (existing.etablissementId !== etablissementId) {
        return NextResponse.json(
          { error: "Cet e-mail est déjà utilisé sur un autre établissement." },
          { status: 409 },
        );
      }
      await db
        .update(user)
        .set({
          firstName: firstName || existing.firstName,
          lastName: lastName || existing.lastName,
          name: `${firstName} ${lastName}`.trim() || existing.name,
          updatedAt: new Date(),
        })
        .where(eq(user.id, existing.id));
      await setUserRolesInDb(existing.id, etablissementId, roles);
      await syncUserAdminFlagsInDb(existing.id, roles);
      return NextResponse.json({
        success: true,
        mode: "better_auth_existing",
        user: {
          externalUserId: existing.externalUserId ?? existing.id,
          email: existing.email,
          firstName: firstName || existing.firstName || undefined,
          lastName: lastName || existing.lastName || undefined,
          roles,
          pending: !existing.emailVerified,
          createdAt: existing.createdAt.toISOString(),
          updatedAt: new Date().toISOString(),
          displayName: `${firstName} ${lastName}`.trim() || existing.name,
        },
        message: "Utilisateur mis à jour. Il peut se connecter ou activer son mot de passe via /auth/sign-up.",
      });
    }

    const id = crypto.randomUUID();
    await db.insert(user).values({
      id,
      email,
      name: `${firstName} ${lastName}`.trim() || email,
      firstName: firstName || null,
      lastName: lastName || null,
      emailVerified: false,
      etablissementId,
      orgAdmin: roles.includes("admin"),
    });
    await setUserRolesInDb(id, etablissementId, roles);
    await syncUserAdminFlagsInDb(id, roles);

    return NextResponse.json({
      success: true,
      mode: "better_auth_created",
      user: {
        externalUserId: id,
        email,
        firstName: firstName || undefined,
        lastName: lastName || undefined,
        roles,
        pending: true,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        displayName: `${firstName} ${lastName}`.trim() || email,
      },
      message:
        "Compte créé. L’utilisateur doit activer son mot de passe sur /auth/sign-up (même e-mail).",
    });
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : "Création impossible";
    console.error("members POST:", e);
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}

export async function PATCH(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const externalUserId = String(body.externalUserId ?? body.userId ?? "").trim();
    const roles = normalizeIntranetRoles(body.intranetRoles ?? body.roles);
    const newEmailRaw = body.email != null ? String(body.email).trim().toLowerCase() : "";
    if (!externalUserId) {
      return NextResponse.json({ error: "userId requis." }, { status: 400 });
    }
    if (roles.some((r) => r === "master")) {
      return NextResponse.json({ error: "Le rôle Master ne peut pas être attribué ici." }, { status: 403 });
    }
    if (roles.length === 0) {
      return NextResponse.json({ error: "Sélectionnez au moins un rôle." }, { status: 400 });
    }
    if (newEmailRaw && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(newEmailRaw)) {
      return NextResponse.json({ error: "Adresse e-mail invalide." }, { status: 400 });
    }
    if (!isDatabaseConfigured()) {
      return NextResponse.json({ error: "Base de données requise." }, { status: 503 });
    }

    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
    const dbUser = await findDbUserByExternalId(etablissementId, externalUserId);
    if (!dbUser) {
      return NextResponse.json({ error: "Utilisateur introuvable." }, { status: 404 });
    }
    const existingRoles = await listMembersFromDb(etablissementId).then(
      (rows) => rows.find((r) => r.externalUserId === (dbUser.externalUserId ?? dbUser.id))?.roles ?? [],
    );
    if (hasMasterRole(existingRoles)) {
      return NextResponse.json({ error: "Ce compte est protégé." }, { status: 403 });
    }

    const emailChanged =
      Boolean(newEmailRaw) && newEmailRaw !== dbUser.email.trim().toLowerCase();
    let email = dbUser.email;
    let emailVerified = dbUser.emailVerified;
    let updatedAt = dbUser.updatedAt;

    if (emailChanged) {
      const db = getDb();
      const [taken] = await db
        .select({ id: user.id, etablissementId: user.etablissementId })
        .from(user)
        .where(and(sql`lower(${user.email}) = ${newEmailRaw}`, ne(user.id, dbUser.id)))
        .limit(1);
      if (taken) {
        return NextResponse.json(
          {
            error:
              taken.etablissementId === etablissementId
                ? "Cet e-mail est déjà utilisé dans cet établissement."
                : "Cet e-mail est déjà utilisé sur un autre établissement.",
          },
          { status: 409 },
        );
      }

      const now = new Date();
      await db
        .update(user)
        .set({
          email: newEmailRaw,
          emailVerified: true,
          updatedAt: now,
        })
        .where(eq(user.id, dbUser.id));

      await db.delete(session).where(eq(session.userId, dbUser.id));

      email = newEmailRaw;
      emailVerified = true;
      updatedAt = now;
    }

    await setUserRolesInDb(dbUser.id, etablissementId, roles);
    await syncUserAdminFlagsInDb(dbUser.id, roles);
    return NextResponse.json({
      success: true,
      emailChanged,
      user: {
        externalUserId: dbUser.externalUserId ?? dbUser.id,
        email,
        firstName: dbUser.firstName ?? undefined,
        lastName: dbUser.lastName ?? undefined,
        roles,
        pending: !emailVerified,
        createdAt: dbUser.createdAt.toISOString(),
        updatedAt: updatedAt.toISOString(),
        displayName: dbUser.name,
      },
    });
  } catch (e) {
    console.error("members PATCH:", e);
    return NextResponse.json({ error: "Mise à jour impossible" }, { status: 500 });
  }
}

export async function DELETE(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const externalUserId = new URL(req.url).searchParams.get("externalUserId")?.trim();
  const email = new URL(req.url).searchParams.get("email")?.trim().toLowerCase();
  if (!externalUserId && !email) {
    return NextResponse.json({ error: "externalUserId ou email requis" }, { status: 400 });
  }
  try {
    if (!isDatabaseConfigured()) {
      return NextResponse.json({ error: "Base de données requise." }, { status: 503 });
    }
    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
    const db = getDb();

    let dbUser =
      (externalUserId ? await findDbUserByExternalId(etablissementId, externalUserId) : null) ?? null;
    if (!dbUser && email) {
      const [byEmail] = await db.select().from(user).where(eq(user.email, email)).limit(1);
      if (byEmail && byEmail.etablissementId === etablissementId) dbUser = byEmail;
    }
    if (!dbUser) {
      return NextResponse.json({ error: "Utilisateur introuvable." }, { status: 404 });
    }

    const members = await listMembersFromDb(etablissementId);
    const roles =
      members.find((m) => m.externalUserId === (dbUser.externalUserId ?? dbUser.id))?.roles ?? [];
    if (hasMasterRole(roles)) {
      return NextResponse.json({ error: "Ce compte est protégé." }, { status: 403 });
    }

    await db.delete(user).where(eq(user.id, dbUser.id));
    return NextResponse.json({ success: true });
  } catch (e) {
    console.error("members DELETE:", e);
    return NextResponse.json({ error: "Suppression impossible" }, { status: 500 });
  }
}

