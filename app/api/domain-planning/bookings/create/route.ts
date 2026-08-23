import { NextResponse, NextRequest } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { getDomainPlanningUserDisplay, isDomainCoordinator } from "@/app/lib/domain-planning-auth";
import { parseCalendarDateLocal } from "@/app/lib/domain-planning-dates";
import { findDomainById, loadBookings, saveBookings } from "@/app/lib/domain-planning-storage";
import type { DomainPlanningBooking } from "@/app/lib/domain-planning-types";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";
import { resolveMemberProfileById } from "@/app/lib/members-db";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

async function resolveDirectoryUserEmail(userId: string): Promise<string> {
  try {
    const profile = await resolveMemberProfileById(userId);
    return profile?.email ?? "";
  } catch {
    return "";
  }
}

function hasConflict(
  existing: DomainPlanningBooking[],
  domainId: string,
  startsAt: string,
  endsAt: string,
): boolean {
  return existing.some(
    (r) =>
      r.domainId === domainId &&
      r.status !== "CANCELLED" &&
      r.startsAt.substring(0, 19) < endsAt &&
      r.endsAt.substring(0, 19) > startsAt,
  );
}

export async function POST(req: NextRequest) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    const authUser = await getDomainPlanningUserDisplay();
    const body = await req.json();
    const {
      domainId,
      selectedHours,
      date,
      className,
      activityLabel,
      comment,
      recurrence,
      untilDate,
      firstName,
      lastName,
      email,
      targetUserId,
    } = body;

    if (!domainId || !date || !Array.isArray(selectedHours) || selectedHours.length === 0 || !className) {
      return NextResponse.json({ error: "Champs obligatoires manquants." }, { status: 400 });
    }

    const domain = await findDomainById(String(domainId).trim());
    if (!domain) return NextResponse.json({ error: "Domaine introuvable." }, { status: 404 });

    const isCoordinator =
      (await isIntranetAdmin()) || (await isDomainCoordinator(authUser.userId, domain.id));

    const existing = await loadBookings();
    const cfg = (await loadAppConfig()).domainPlanning;
    const limitDate = new Date();
    limitDate.setDate(limitDate.getDate() + (cfg.bookingHorizonDays || 56));

    const assigningOther =
      isCoordinator &&
      ((targetUserId?.trim() && targetUserId.trim() !== authUser.userId) ||
        (firstName?.trim() &&
          lastName?.trim() &&
          (firstName.trim() !== authUser.firstName || lastName.trim().toUpperCase() !== authUser.lastName)));
    const assignmentKind: "coordinator" | "self" = assigningOther ? "coordinator" : "self";

    const bookingUserId =
      assignmentKind === "coordinator" && targetUserId?.trim()
        ? String(targetUserId).trim()
        : assignmentKind === "coordinator"
          ? `assigned-${Date.now()}`
          : authUser.userId;

    const bookingFirstName =
      assignmentKind === "coordinator" && firstName?.trim() ? String(firstName).trim() : authUser.firstName;
    const bookingLastName =
      assignmentKind === "coordinator" && lastName?.trim()
        ? String(lastName).trim().toUpperCase()
        : authUser.lastName;
    let bookingEmail = email?.trim() || "";
    if (assignmentKind === "coordinator") {
      if (!bookingEmail && targetUserId?.trim()) {
        bookingEmail = await resolveDirectoryUserEmail(String(targetUserId).trim());
      }
      if (!bookingEmail) {
        return NextResponse.json(
          { error: "E-mail du professeur introuvable pour la confirmation." },
          { status: 400 },
        );
      }
    } else {
      bookingEmail = bookingEmail || authUser.email;
    }

    if (assignmentKind === "self" && bookingUserId !== authUser.userId) {
      return NextResponse.json({ error: "Réservation personnelle uniquement." }, { status: 403 });
    }

    const newBookings: DomainPlanningBooking[] = [];
    const groupId = recurrence !== "none" ? `group-${Date.now()}-${Math.random().toString(36).slice(2, 7)}` : null;

    for (const hour of selectedHours as number[]) {
      const currentLoopDate = parseCalendarDateLocal(String(date));
      const stopDate =
        recurrence !== "none" && untilDate
          ? parseCalendarDateLocal(String(untilDate))
          : parseCalendarDateLocal(String(date));
      stopDate.setHours(23, 59, 59, 999);

      while (currentLoopDate <= stopDate) {
        if (!isCoordinator && currentLoopDate > limitDate) break;
        const dateStr = currentLoopDate.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
        const startsAt = `${dateStr}T${hour.toString().padStart(2, "0")}:30:00`;
        const endsAt = `${dateStr}T${(hour + 1).toString().padStart(2, "0")}:30:00`;

        if (!hasConflict(existing, domain.id, startsAt, endsAt)) {
          const booking: DomainPlanningBooking = {
            id: `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
            groupId,
            domainId: domain.id,
            userId: bookingUserId,
            firstName: bookingFirstName,
            lastName: bookingLastName,
            email: bookingEmail,
            className: String(className).trim(),
            activityLabel: activityLabel?.trim() || undefined,
            comment: String(comment || "").trim(),
            startsAt,
            endsAt,
            createdAt: new Date().toISOString(),
            createdByUserId: authUser.userId,
            assignmentKind,
            status: "CONFIRMED",
          };
          newBookings.push(booking);
          existing.push(booking);
        }

        if (recurrence === "weekly") currentLoopDate.setDate(currentLoopDate.getDate() + 7);
        else if (recurrence === "biweekly") currentLoopDate.setDate(currentLoopDate.getDate() + 14);
        else break;
      }
    }

    if (newBookings.length === 0) {
      return NextResponse.json({ error: "Aucun créneau disponible." }, { status: 409 });
    }

    await saveBookings(existing);

    if (bookingEmail) {
      const smtp = await getTenantSmtpConfig();
      const transporter = smtp ? await createTenantTransporter() : null;
      if (transporter && smtp) {
        const datesList = newBookings
          .map((r) => {
            const d = parseCalendarDateLocal(r.startsAt.split("T")[0]);
            const dateFr = d.toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long" });
            const hourFr = r.startsAt.split("T")[1].substring(0, 5).replace(":", "h");
            return `<li>Le ${dateFr} à ${hourFr}</li>`;
          })
          .join("");
        await transporter.sendMail({
          from: `"Enseignements transversaux" <${smtp.user}>`,
          to: bookingEmail,
          subject: `Confirmation — ${domain.name}`,
          html: `
            <div style="font-family:sans-serif;max-width:600px;margin:0 auto;">
              <h2 style="color:#7c3aed;">Créneau confirmé — ${domain.name}</h2>
              <p>Bonjour ${bookingFirstName},</p>
              <ul>${datesList}</ul>
              <p>Classe : <strong>${className}</strong>${activityLabel ? ` — ${activityLabel}` : ""}</p>
            </div>
          `,
        });
      }
    }

    return NextResponse.json({ success: true, count: newBookings.length }, { status: 201 });
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : "Erreur serveur";
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}
