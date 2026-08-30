import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { loadAppConfig, saveNotifications } from "@/app/lib/app-config";
import { parseNotifications, type AbsenceNotifyPerson } from "@/app/lib/app-config-schemas";
import {
  viewerCanConfigureAbsenceProcessors,
  viewerCanSeeProcessorQueue,
} from "@/app/lib/absences-admin-access";
import { listDirectoryMembers } from "@/app/lib/directory-members";

function asPerson(value: unknown): AbsenceNotifyPerson | null {
  if (!value || typeof value !== "object") return null;
  const email = String((value as { email?: string }).email || "").trim();
  if (!email) return null;
  const label = String((value as { label?: string }).label || "").trim() || undefined;
  const userId = String((value as { userId?: string }).userId || "").trim() || undefined;
  return { email, label, userId };
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const viewer = {
    email: user?.primaryEmailAddress?.emailAddress || "",
    userId: user?.id || "",
    roles,
  };
  const bundle = await loadAppConfig();
  const n = bundle.notifications;
  const canConfigure = viewerCanConfigureAbsenceProcessors(roles);
  const canSeeQueue = viewerCanSeeProcessorQueue(viewer, n);

  let members: Array<{
    externalUserId: string;
    email: string;
    firstName?: string;
    lastName?: string;
    displayName?: string;
  }> = [];
  if (canConfigure) {
    const rows = await listDirectoryMembers();
    members = rows
      .filter((u) => u.externalUserId && !u.pending)
      .map((u) => ({
        externalUserId: u.externalUserId,
        email: u.email,
        firstName: u.firstName,
        lastName: u.lastName,
        displayName: u.displayName,
      }));
  }

  return NextResponse.json({
    viewerIsProcessor: canSeeQueue,
    viewerCanConfigure: canConfigure,
    processors:
      canSeeQueue || canConfigure
        ? {
            absencesNotifyProfEcole: n.absencesNotifyProfEcole ?? null,
            absencesNotifyProfCollege: n.absencesNotifyProfCollege ?? n.absencesNotifyProfCollegeLycee ?? null,
            absencesNotifyProfLycee: n.absencesNotifyProfLycee ?? n.absencesNotifyProfCollegeLycee ?? null,
            absencesNotifyOgecCompta: n.absencesNotifyOgecCompta ?? [],
          }
        : null,
    members,
  });
}

export async function PUT(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!viewerCanConfigureAbsenceProcessors(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  const body = (await req.json()) as Record<string, unknown>;
  const current = await loadAppConfig();
  const n = current.notifications;
  const has = (key: string) => Object.prototype.hasOwnProperty.call(body, key);
  const next = parseNotifications({
    ...n,
    absencesNotifyProfEcole: has("absencesNotifyProfEcole")
      ? (asPerson(body.absencesNotifyProfEcole) ?? undefined)
      : n.absencesNotifyProfEcole,
    absencesNotifyProfCollege: has("absencesNotifyProfCollege")
      ? (asPerson(body.absencesNotifyProfCollege) ?? undefined)
      : n.absencesNotifyProfCollege,
    absencesNotifyProfLycee: has("absencesNotifyProfLycee")
      ? (asPerson(body.absencesNotifyProfLycee) ?? undefined)
      : n.absencesNotifyProfLycee,
    absencesNotifyOgecCompta: Array.isArray(body.absencesNotifyOgecCompta)
      ? body.absencesNotifyOgecCompta
      : n.absencesNotifyOgecCompta,
  });
  await saveNotifications(next);
  return NextResponse.json({ ok: true, notifications: next });
}
