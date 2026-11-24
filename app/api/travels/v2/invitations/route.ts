import { after, NextResponse } from "next/server";
import { z } from "zod";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canSignTravelsDirectionForEtab, isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import {
  isGlobalAdminUser,
  userHasAdministratifRole,
  userHasComptaRoleFromMetadata,
} from "@/app/lib/travels-roles";
import { getTenant, getTenantAppUrl, runWithTenant } from "@/app/lib/tenant-context";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { isIsoDate, isWorkflowV2, todayIsoParis, tripDepartureIso } from "@/app/lib/travels-v2";
import {
  buildRegistrationRows,
  createInvitationsForTrip,
  getInvitation,
  listInvitations,
  saveInvitation,
  sendInvitationLink,
  sendPendingInvitations,
  signedUrlForInvitationFile,
} from "@/app/lib/travels-v2-registration-store";
import { computeInvitationStatus } from "@/app/lib/travels-v2-registration-types";
import { listPicnicDays } from "@/app/lib/travels-family-page";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { loadAppConfig } from "@/app/lib/app-config";

/**
 * Vue « Inscriptions » du hub voyage (circuit v2) — API INTERNE (compte requis).
 * Lecture : organisateur, direction de l'établissement, administratif, admin général, compta.
 * Actions : organisateur, direction, administratif, admin général (la compta consulte seulement).
 */

type Access = {
  isOwner: boolean;
  isDirection: boolean;
  isAdmin: boolean;
  isAdministratif: boolean;
  isCompta: boolean;
  userName: string;
};

async function resolveAccess(trip: NonNullable<Awaited<ReturnType<typeof getTravelTrip>>>): Promise<Access | null> {
  const me = await safeCurrentUser();
  if (!me) return null;
  const meta = me.publicMetadata as Record<string, unknown> | undefined;
  return {
    isOwner: isTripOwnerOrCreator(trip, me),
    isDirection: Boolean(await canSignTravelsDirectionForEtab(me, trip.data?.etablissement)),
    isAdmin: isGlobalAdminUser({ publicMetadata: meta }),
    isAdministratif: userHasAdministratifRole({ publicMetadata: meta }),
    isCompta: userHasComptaRoleFromMetadata(meta),
    userName: me.fullName || "Utilisateur",
  };
}

const canRead = (a: Access) => a.isOwner || a.isDirection || a.isAdmin || a.isAdministratif || a.isCompta;
const canWrite = (a: Access) => a.isOwner || a.isDirection || a.isAdmin || a.isAdministratif;
const canSeeAmount = (a: Access) => a.isCompta || a.isDirection || a.isAdmin;

async function loadV2Trip(tripId: string) {
  const trip = await getTravelTrip(tripId);
  if (!trip || !isWorkflowV2(trip)) return null;
  return trip;
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const tripId = new URL(req.url).searchParams.get("tripId")?.trim() || "";
  if (!tripId) return NextResponse.json({ error: "Voyage manquant." }, { status: 400 });
  try {
    const trip = await loadV2Trip(tripId);
    if (!trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    const access = await resolveAccess(trip);
    if (!access || !canRead(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    const opened = Boolean(trip.data?.familyPage?.openedAt);
    const invitations = opened ? await listInvitations(tripId) : [];
    const rows = buildRegistrationRows({ trip, invitations, includeAmount: canSeeAmount(access) });
    const docs = trip.data?.familyPage?.documents || [];
    const keys = listPicnicDays(trip.data?.piqueNiqueDetails).map((d) => d.key);
    const counts: Record<string, number> = {};
    for (const inv of invitations) {
      const s = computeInvitationStatus({ inv, docs, picnicDayKeys: keys });
      counts[s] = (counts[s] || 0) + 1;
    }
    // Élèves inscrits = élèves distincts avec au moins un « oui » signé (un second parent ne double pas l'engagement).
    const engaged = new Map<string, number>();
    for (const inv of invitations) {
      if (inv.signature?.decision === "oui" && inv.engagement && !engaged.has(inv.ine)) {
        engaged.set(inv.ine, inv.engagement.amount);
      }
    }
    const smtpOk = Boolean(await getTenantSmtpConfig());
    return NextResponse.json({
      opened,
      status: trip.status,
      deadline: trip.data?.familyPage?.deadline ?? null,
      rows,
      counts,
      engagedStudents: engaged.size,
      ...(canSeeAmount(access) ? { engagedTotal: [...engaged.values()].reduce((s, n) => s + n, 0) } : {}),
      canWrite: canWrite(access),
      canSeeAmount: canSeeAmount(access),
      smtpOk,
    });
  } catch (e) {
    console.error("[travels/v2/invitations] GET", e);
    return NextResponse.json({ error: "Chargement impossible." }, { status: 500 });
  }
}

const actionSchema = z.discriminatedUnion("action", [
  z.object({ action: z.literal("send_pending"), tripId: z.string().min(1) }),
  z.object({ action: z.literal("create_missing"), tripId: z.string().min(1) }),
  z.object({ action: z.literal("resend"), tripId: z.string().min(1), invitationId: z.string().min(1) }),
  z.object({
    action: z.literal("exempt"),
    tripId: z.string().min(1),
    invitationId: z.string().min(1),
    docId: z.string().min(1),
    value: z.boolean(),
  }),
  z.object({
    action: z.literal("extend"),
    tripId: z.string().min(1),
    invitationId: z.string().min(1),
    date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  }),
  z.object({
    action: z.literal("download"),
    tripId: z.string().min(1),
    invitationId: z.string().min(1),
    docId: z.string().min(1).optional(),
    engagement: z.boolean().optional(),
  }),
]);

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const parsed = actionSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ error: "Requête invalide." }, { status: 400 });
  const input = parsed.data;
  try {
    const trip = await loadV2Trip(input.tripId);
    if (!trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    if (!trip.data?.familyPage?.openedAt) {
      return NextResponse.json({ error: "La page des familles n'est pas encore ouverte." }, { status: 409 });
    }
    const access = await resolveAccess(trip);
    if (!access || !canRead(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });

    if (input.action === "download") {
      const inv = await getInvitation(input.tripId, input.invitationId);
      if (!inv) return NextResponse.json({ error: "Introuvable." }, { status: 404 });
      if (input.engagement) {
        // Engagement signé : compta, direction, admin, organisateur, administratif.
        const url = await signedUrlForInvitationFile(inv, { kind: "engagement" });
        return url ? NextResponse.json({ url }) : NextResponse.json({ error: "Aucun engagement signé." }, { status: 404 });
      }
      // Pièces déposées par les familles : pas pour la compta seule.
      if (!canWrite(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
      if (!input.docId) return NextResponse.json({ error: "Pièce manquante." }, { status: 400 });
      const url = await signedUrlForInvitationFile(inv, { kind: "document", docId: input.docId });
      return url ? NextResponse.json({ url }) : NextResponse.json({ error: "Pièce introuvable." }, { status: 404 });
    }

    if (!canWrite(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });

    if (trip.status !== "INSCRIPTIONS_OUVERTES") {
      return NextResponse.json({ error: "Les inscriptions ne sont plus ouvertes pour ce voyage." }, { status: 409 });
    }

    if (input.action === "send_pending" || input.action === "create_missing") {
      const tenant = await getTenant();
      const baseUrl = await getTenantAppUrl();
      const tripCopy = trip;
      after(async () => {
        try {
          await runWithTenant(tenant, async () => {
            if (input.action === "create_missing") await createInvitationsForTrip(tripCopy);
            await sendPendingInvitations({ trip: tripCopy, baseUrl });
          });
        } catch (e) {
          console.error("[travels/v2/invitations] envoi", e instanceof Error ? e.message : e);
        }
      });
      return NextResponse.json({ ok: true, queued: true });
    }

    const inv = await getInvitation(input.tripId, input.invitationId);
    if (!inv) return NextResponse.json({ error: "Introuvable." }, { status: 404 });

    if (input.action === "resend") {
      if (inv.signature) return NextResponse.json({ error: "Cette famille a déjà signé." }, { status: 409 });
      const smtp = await getTenantSmtpConfig();
      const transporter = smtp ? await createTenantTransporter() : null;
      if (!smtp || !transporter) return NextResponse.json({ error: "Envoi d'e-mails indisponible." }, { status: 503 });
      const bundle = await loadAppConfig().catch(() => null);
      const school = bundle?.identity?.shortName || bundle?.identity?.name || "L'établissement";
      const ok = await sendInvitationLink({
        trip,
        inv,
        baseUrl: await getTenantAppUrl(),
        mail: { smtp, transporter, school },
      });
      return ok ? NextResponse.json({ ok: true }) : NextResponse.json({ error: "L'e-mail n'a pas pu partir." }, { status: 502 });
    }

    if (input.action === "exempt") {
      const doc = (trip.data?.familyPage?.documents || []).find((d) => d.id === input.docId);
      if (!doc) return NextResponse.json({ error: "Pièce inconnue." }, { status: 400 });
      if (inv.signature) return NextResponse.json({ error: "Cette famille a déjà signé." }, { status: 409 });
      const next = { ...(inv.exemptions || {}) };
      if (input.value) next[input.docId] = { at: new Date().toISOString(), by: access.userName };
      else delete next[input.docId];
      inv.exemptions = next;
      await saveInvitation(inv);
      return NextResponse.json({ ok: true });
    }

    if (input.action === "extend") {
      const departure = tripDepartureIso(trip.data);
      if (!isIsoDate(input.date) || input.date < todayIsoParis() || (departure && input.date >= departure)) {
        return NextResponse.json({ error: "Date invalide : elle doit être future et avant le départ." }, { status: 400 });
      }
      if (inv.signature) return NextResponse.json({ error: "Cette famille a déjà signé." }, { status: 409 });
      inv.extendedDeadline = input.date;
      await saveInvitation(inv);
      return NextResponse.json({ ok: true });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    console.error("[travels/v2/invitations] POST", e);
    return NextResponse.json({ error: "Action impossible." }, { status: 500 });
  }
}
