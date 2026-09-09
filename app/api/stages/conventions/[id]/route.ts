import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewAllConventions, canViewReferentConventions } from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import {
  approveDepositedConvention,
  addConventionSignatory,
  markConventionSignatureManual,
  normalizeConventionInput,
  removeConventionSignatory,
  reviewConventionSignature,
  reviewPreconvention,
  submitPreconvention,
  syncProfReferentSignatory,
} from "@/app/lib/stage-workflow";
import { getStageConvention, saveStageConvention } from "@/app/lib/stage-storage";
import { ensureConventionReferent, listClassesForReferentUser, userCanAssignStageReferentForClass } from "@/app/lib/stage-referents-config";
import {
  getStageWatchersConfig,
  listWatcherAssignmentsForUser,
} from "@/app/lib/stage-watchers-config";
import { notifyAllStageSignatureRequests, notifyStageDepositAdminRejected, notifyStageSignatureRequest } from "@/app/lib/stage-notify";
import type { StageSignerRole } from "@/app/lib/stage-types";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
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
    const referentClassNames = canViewReferentConventions(roles)
      ? await listClassesForReferentUser(gate.ctx.userId)
      : [];
    const watchers = await getStageWatchersConfig(convention.schoolYear || currentStageSchoolYear());
    const watcherAssignments = listWatcherAssignmentsForUser(watchers, gate.ctx.userId);
    if (
      !conventionVisibleToUser(
        convention,
        roles,
        userEmail,
        gate.ctx.userId,
        referentClassNames,
        watcherAssignments,
      ) &&
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

    if (action === "assign_referent") {
      const allowed =
        canReviewPreconvention(roles) ||
        (await userCanAssignStageReferentForClass(
          gate.ctx.userId,
          convention.student.className,
          convention.schoolYear,
        ));
      if (!allowed) {
        return NextResponse.json(
          { error: "Seul le professeur principal de la classe (ou l'administratif) peut déléguer un référent." },
          { status: 403 },
        );
      }
      const externalUserId = String(body.externalUserId ?? "").trim();
      const name = String(body.name ?? "").trim();
      const email = String(body.email ?? "").trim().toLowerCase();
      if (!externalUserId || !name || !email) {
        return NextResponse.json(
          { error: "Choisissez un professeur référent (nom et e-mail requis)." },
          { status: 400 },
        );
      }
      const now = new Date().toISOString();
      convention = {
        ...convention,
        teacherReferent: { name, email, userId: externalUserId },
        updatedAt: now,
        history: [
          ...convention.history,
          {
            at: now,
            by: displayName(user),
            action: "REFERENT_DELEGUE",
            note: `${name} <${email}>`,
          },
        ],
      };
      if (convention.status === "signatures_pending" || convention.status === "convention_ready") {
        convention = await syncProfReferentSignatory(convention, {
          name,
          email,
          userId: externalUserId,
          byName: displayName(user),
        });
      } else {
        await saveStageConvention(convention);
      }
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

    if (action === "review_signature") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const signatureId = String(body.signatureId ?? "").trim();
      if (!signatureId) {
        return NextResponse.json({ error: "signatureId requis." }, { status: 400 });
      }
      const result = await reviewConventionSignature({
        conventionId: convention.id,
        signatureId,
        accepted: body.accepted === true,
        by: gate.ctx.userId,
        byName: displayName(user),
        note: String(body.note ?? "").trim() || undefined,
      });
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    if (action === "resend_signature") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      if (convention.status !== "signatures_pending") {
        return NextResponse.json({ error: "Aucune signature en attente." }, { status: 400 });
      }
      const signatureId = String(body.signatureId ?? "").trim();
      const sig = convention.signatures.find((s) => s.id === signatureId);
      if (!sig || sig.status !== "en_attente") {
        return NextResponse.json({ error: "Signature introuvable ou déjà signée." }, { status: 400 });
      }
      const mail = await notifyStageSignatureRequest(convention, sig);
      return NextResponse.json({
        success: true,
        mail,
        role: sig.role,
        email: sig.signEmail,
      });
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

    if (action === "add_signatory") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const role = String(body.role ?? "").trim() as StageSignerRole;
      const email = String(body.email ?? "").trim();
      const name = String(body.name ?? "").trim() || undefined;
      const result = await addConventionSignatory({
        conventionId: convention.id,
        role,
        email,
        name,
        byName: displayName(user),
      });
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    if (action === "remove_signatory") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const signatureId = String(body.signatureId ?? "").trim();
      if (!signatureId) {
        return NextResponse.json({ error: "signatureId requis." }, { status: 400 });
      }
      const result = await removeConventionSignatory({
        conventionId: convention.id,
        signatureId,
        byName: displayName(user),
      });
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    if (action === "mark_signature_manual") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const signatureId = String(body.signatureId ?? "").trim();
      if (!signatureId) {
        return NextResponse.json({ error: "signatureId requis." }, { status: 400 });
      }
      const result = await markConventionSignatureManual({
        conventionId: convention.id,
        signatureId,
        byName: displayName(user),
        note: String(body.note ?? "").trim() || undefined,
      });
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    if (action === "file_eleve_dossier") {
      if (!canReviewPreconvention(roles)) {
        return NextResponse.json({ error: "Réservé à l'administratif / direction." }, { status: 403 });
      }
      const { fileSignedConventionToEleveDossier } = await import(
        "@/app/lib/stage-eleve-dossier-filing"
      );
      const result = await fileSignedConventionToEleveDossier(convention, displayName(user));
      if (!result.ok) {
        return NextResponse.json({ error: result.error }, { status: 400 });
      }
      return NextResponse.json({
        success: true,
        convention: result.convention,
        eleveDossier: {
          eleveId: result.eleveId,
          documentId: result.documentId,
          dossierUrl: `/eleves/dossier/${result.eleveId}`,
        },
      });
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
