import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewAllConventions, canViewReferentConventions } from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import {
  approveDepositedConvention,
  normalizeConventionInput,
  reviewPreconvention,
  submitPreconvention,
} from "@/app/lib/stage-workflow";
import { getStageConvention, saveStageConvention } from "@/app/lib/stage-storage";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import { notifyAllStageSignatureRequests, notifyStageDepositAdminRejected } from "@/app/lib/stage-notify";
import {
  findEleveByIne,
  matchEleveForConvention,
  resolveConventionSecteur,
  resolveOneDriveProfileForConvention,
} from "@/app/lib/stage-eleve-match";
import { resolveOneDriveProfileForUserServer } from "@/app/lib/onedrive-user-profiles.server";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Utilisateur";
}

export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const { id } = await ctx.params;
    const convention = await getStageConvention(id);
    if (!convention) return NextResponse.json({ error: "Convention introuvable." }, { status: 404 });

    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    if (
      !conventionVisibleToUser(convention, roles, userEmail, gate.ctx.userId) &&
      !roles.includes("parent")
    ) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const signLinks = convention.signatures
      .filter((s) => s.signToken && s.status === "en_attente")
      .map((s) => ({
        role: s.role,
        label: s.label,
        email: s.signEmail,
        link: `/stages/signer?token=${encodeURIComponent(s.signToken!)}`,
      }));

    const directoryProfile = user ? await resolveOneDriveProfileForUserServer(user) : null;
    const targetProfile = await resolveOneDriveProfileForConvention(convention, directoryProfile);
    const eleveMatch = await matchEleveForConvention(convention, targetProfile);
    const conventionSecteur = await resolveConventionSecteur(convention);

    return NextResponse.json({
      convention,
      studentLink: convention.studentAccessToken
        ? `/stages/eleve?token=${encodeURIComponent(convention.studentAccessToken)}`
        : null,
      signLinks,
      eleveMatch: {
        matchedEleve: eleveMatch.matchedEleve
          ? {
              ine: eleveMatch.matchedEleve.ine,
              nom: eleveMatch.matchedEleve.nom,
              prenom: eleveMatch.matchedEleve.prenom,
              folderName: eleveMatch.matchedEleve.folderName,
            }
          : null,
        folderPath: eleveMatch.folderPath,
        secteur: conventionSecteur,
        targetOneDriveLabel: targetProfile?.label ?? null,
        debug: eleveMatch.debug,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function PATCH(req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const { id } = await ctx.params;
    let convention = await getStageConvention(id);
    if (!convention) return NextResponse.json({ error: "Convention introuvable." }, { status: 404 });

    const body = await req.json();
    const action = String(body.action ?? "save");

    if (action === "save") {
      convention = normalizeConventionInput(body.convention ?? body, convention);
      if (convention.status === "admin_review" && !canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      convention = await ensureConventionReferent(convention);
      const now = new Date().toISOString();
      if (canReviewPreconvention(roles) && convention.status === "admin_review") {
        convention = {
          ...convention,
          updatedAt: now,
          history: [
            ...convention.history,
            { at: now, by: displayName(user), action: "ADMIN_MODIFIE" },
          ],
        };
      }
      await saveStageConvention(convention);
      return NextResponse.json({ success: true, convention });
    }

    if (action === "submit") {
      convention = normalizeConventionInput(body.convention ?? body, convention);
      const result = await submitPreconvention(convention, displayName(user));
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    if (action === "admin_review") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const approved = body.approved === true;
      if (convention.status === "convention_deposited" && approved) {
        const result = await approveDepositedConvention(convention, {
          by: gate.ctx.userId,
          byName: displayName(user),
          note: String(body.note ?? "").trim() || undefined,
        });
        if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
        const next = result.convention;
        const signLinks = next.signatures
          .filter((s) => s.signToken)
          .map((s) => ({
            role: s.role,
            label: s.label,
            email: s.signEmail,
            link: `/stages/signer?token=${encodeURIComponent(s.signToken!)}`,
          }));
        return NextResponse.json({ success: true, convention: next, signLinks });
      }
      if (convention.status === "convention_deposited" && !approved) {
        const now = new Date().toISOString();
        const note = String(body.note ?? "").trim() || undefined;
        const next = {
          ...convention,
          status: "cancelled" as const,
          updatedAt: now,
          adminReview: {
            at: now,
            by: gate.ctx.userId,
            byName: displayName(user),
            approved: false,
            note,
          },
          history: [
            ...convention.history,
            { at: now, by: displayName(user), action: "DEPOT_REFUSE", note },
          ],
        };
        await saveStageConvention(next);
        void notifyStageDepositAdminRejected(next, note).catch((e) =>
          console.error("[stages] notify depot reject:", e),
        );
        return NextResponse.json({ success: true, convention: next, signLinks: [] });
      }
      try {
        const next = await reviewPreconvention(convention, {
          by: gate.ctx.userId,
          byName: displayName(user),
          approved,
          note: String(body.note ?? "").trim() || undefined,
        });
        const signLinks = next.signatures
          .filter((s) => s.signToken)
          .map((s) => ({
            role: s.role,
            label: s.label,
            email: s.signEmail,
            secureCode: s.signSecureCode,
            link: `/stages/signer?token=${encodeURIComponent(s.signToken!)}`,
          }));
        return NextResponse.json({ success: true, convention: next, signLinks });
      } catch (e: unknown) {
        return NextResponse.json(
          { error: e instanceof Error ? e.message : "Erreur validation préconvention." },
          { status: 400 },
        );
      }
    }

    if (action === "attach_eleve") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const ine = String(body.matchedEleveIne ?? "").trim();
      if (ine) {
        const found = await findEleveByIne(ine);
        if (!found) {
          return NextResponse.json(
            { error: `INE introuvable dans eleves.json : ${ine}` },
            { status: 400 },
          );
        }
      }
      const now = new Date().toISOString();
      convention = {
        ...convention,
        updatedAt: now,
        ocrMeta: {
          extractedAt: convention.ocrMeta?.extractedAt ?? now,
          matchedEleveIne: ine || undefined,
          matchScore: ine ? 4 : convention.ocrMeta?.matchScore,
          raw: convention.ocrMeta?.raw,
        },
        history: [
          ...convention.history,
          {
            at: now,
            by: displayName(user),
            action: "ELEVE_RATTACHE",
            note: ine || "Rattachement retiré",
          },
        ],
      };
      await saveStageConvention(convention);
      return NextResponse.json({ success: true, convention });
    }

    if (action === "resend_signatures") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      if (convention.status !== "signatures_pending") {
        return NextResponse.json({ error: "Aucune signature en attente." }, { status: 400 });
      }
      const mail = await notifyAllStageSignatureRequests(convention);
      return NextResponse.json({ success: true, mail });
    }

    if (action === "cancel") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      convention = {
        ...convention,
        status: "cancelled",
        updatedAt: new Date().toISOString(),
        history: [
          ...convention.history,
          {
            at: new Date().toISOString(),
            by: displayName(user),
            action: "ANNULEE",
            note: String(body.note ?? "").trim() || undefined,
          },
        ],
      };
      await saveStageConvention(convention);
      return NextResponse.json({ success: true, convention });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
