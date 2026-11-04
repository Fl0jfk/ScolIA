import { NextResponse } from "next/server";
import { requireAuth, requireTenantAdminRole } from "@/app/lib/intranet-auth";
import { getDb } from "@/db/index";
import { user, userRole } from "@/db/schema";
import { and, eq, inArray, notInArray } from "drizzle-orm";
import { setUserBirthDate } from "@/app/lib/user-birthdate";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { getAllPersonnelRecords, getPersonnelRecord, savePersonnelRecord } from "@/app/lib/personnel-storage";
import { canManagePersonnel, normalizePersonnelRecord } from "@/app/lib/personnel-types";
import { normalizePersonnelProfile } from "@/app/lib/personnel-profile";
import {
  BIRTHDATE_IMPORT_MAX_BYTES,
  matchBirthdateRows,
  nameKey,
  parseBirthdateSheetBuffer,
  summarizeBirthdateRows,
  type BirthdateStaff,
} from "@/app/lib/personnel-birthdate-import";

export const dynamic = "force-dynamic";

function isAllowedFile(file: File) {
  const n = file.name.toLowerCase();
  return n.endsWith(".xlsx") || n.endsWith(".xls") || n.endsWith(".csv");
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const sessionUser = await safeCurrentUser();
  if (!canManagePersonnel(rolesFromUserLike(sessionUser))) {
    // Administrateurs d'établissement : l'import concerne désormais tous les comptes internes.
    const admin = await requireTenantAdminRole();
    if (!admin.ok) return admin.response;
  }
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  try {
    const form = await req.formData();
    const file = form.get("file");
    const mode = String(form.get("mode") || "preview");
    if (mode !== "preview" && mode !== "apply") {
      return NextResponse.json({ error: "Mode inconnu." }, { status: 400 });
    }
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier requis." }, { status: 400 });
    }
    if (!isAllowedFile(file)) {
      return NextResponse.json({ error: "Format .xlsx, .xls ou .csv requis." }, { status: 400 });
    }
    if (file.size > BIRTHDATE_IMPORT_MAX_BYTES) {
      return NextResponse.json({ error: "Fichier trop volumineux (2 Mo maximum)." }, { status: 413 });
    }

    const parsed = parseBirthdateSheetBuffer(await file.arrayBuffer());
    if (!parsed.ok) return NextResponse.json({ error: parsed.error }, { status: 400 });

    const etablissementId = tenant.ctx.etablissementId;
    const lc = (v: string | null | undefined) => (v ?? "").trim().toLowerCase();
    const db = getDb();

    // Comptes internes de l'établissement (au moins un rôle autre que parent/eleve).
    const internalIds = db
      .select({ id: userRole.userId })
      .from(userRole)
      .where(and(eq(userRole.etablissementId, etablissementId), notInArray(userRole.role, ["parent", "eleve"])));
    const accounts = await db
      .select({
        id: user.id,
        externalUserId: user.externalUserId,
        email: user.email,
        name: user.name,
        firstName: user.firstName,
        lastName: user.lastName,
        dateNaissance: user.dateNaissance,
      })
      .from(user)
      .where(and(eq(user.etablissementId, etablissementId), inArray(user.id, internalIds)));

    // Fiches du personnel (getAllPersonnelRecords est scopé à l'établissement de la session).
    const allRecords = await getAllPersonnelRecords();
    const ficheEmails = (r: (typeof allRecords)[number]) =>
      [r.email, r.emailPro, r.emailPerso].filter((x): x is string => !!x);
    const linkedFiche = new Map<string, (typeof allRecords)[number]>(); // user.id -> fiche
    const linkedFicheIds = new Set<string>();
    for (const a of accounts) {
      const mail = lc(a.email);
      const f = allRecords.find(
        (r) =>
          (r.externalUserId && (r.externalUserId === a.id || r.externalUserId === a.externalUserId)) ||
          (mail && ficheEmails(r).some((e) => lc(e) === mail)),
      );
      if (f) {
        linkedFiche.set(a.id, f);
        linkedFicheIds.add(f.id);
      }
    }
    // Repli par nom normalisé : exactement UNE fiche active non rattachée et UN compte non rattaché de ce nom.
    const acctByName = new Map<string, typeof accounts>();
    for (const a of accounts) {
      if (linkedFiche.has(a.id)) continue;
      const k = nameKey(a.firstName ?? "", a.lastName ?? "") || nameKey(a.name ?? "");
      if (k) acctByName.set(k, [...(acctByName.get(k) ?? []), a]);
    }
    const ficheByName = new Map<string, typeof allRecords>();
    for (const r of allRecords) {
      if (r.active === false || linkedFicheIds.has(r.id)) continue;
      const k = nameKey(r.firstName, r.lastName);
      if (k) ficheByName.set(k, [...(ficheByName.get(k) ?? []), r]);
    }
    for (const [k, accs] of acctByName) {
      const fs = ficheByName.get(k);
      if (accs.length === 1 && fs && fs.length === 1) {
        linkedFiche.set(accs[0].id, fs[0]);
        linkedFicheIds.add(fs[0].id);
      }
    }

    const staff: BirthdateStaff[] = [
      ...accounts.map((a) => {
        const f = linkedFiche.get(a.id);
        return {
          id: `u:${a.id}`,
          firstName: (a.firstName ?? "").trim() || f?.firstName || (a.name ?? "").trim(),
          lastName: (a.lastName ?? "").trim() || f?.lastName || "",
          emails: [a.email, ...(f ? ficheEmails(f) : [])].filter(Boolean),
          birthDate: a.dateNaissance || f?.profile?.birthDate || null,
        };
      }),
      ...allRecords
        .filter((r) => r.active !== false && !linkedFicheIds.has(r.id))
        .map((r) => ({
          id: `p:${r.id}`,
          firstName: r.firstName,
          lastName: r.lastName,
          emails: ficheEmails(r),
          birthDate: r.profile?.birthDate || null,
        })),
    ];

    const rows = matchBirthdateRows(parsed.rows, staff);
    const counts = summarizeBirthdateRows(rows);

    let written = 0;
    let failed = 0;
    const writeFiche = async (ficheId: string, date: string) => {
      // Relecture juste avant l'écriture : ne pas écraser une modification faite entre-temps.
      const rec = await getPersonnelRecord(ficheId);
      if (!rec) return;
      await savePersonnelRecord(
        normalizePersonnelRecord({
          ...rec,
          profile: normalizePersonnelProfile({ ...(rec.profile || {}), birthDate: date }),
          updatedAt: new Date().toISOString(),
        }),
      );
    };
    if (mode === "apply") {
      const staffIds = new Set(staff.map((s) => s.id));
      for (const row of rows) {
        if ((row.status !== "ok" && row.status !== "remplace") || !row.staffId || !row.newDate) continue;
        if (!staffIds.has(row.staffId)) continue;
        try {
          if (row.staffId.startsWith("u:")) {
            const userId = row.staffId.slice(2);
            const done = await setUserBirthDate(etablissementId, userId, row.newDate);
            if (!done) {
              failed++;
              continue;
            }
            const f = linkedFiche.get(userId);
            if (f) await writeFiche(f.id, row.newDate);
          } else {
            await writeFiche(row.staffId.slice(2), row.newDate);
          }
          written++;
        } catch {
          failed++;
        }
      }
      await writeDataAccessAudit({
        etablissementId,
        userId: tenant.ctx.authUserId,
        resourceType: "members",
        action: "import",
        req,
        metadata: { kind: "birthdates", lignes: rows.length, ecrites: written, echecs: failed, ...counts },
      });
    }

    return NextResponse.json({
      mode,
      counts,
      written,
      failed,
      rows: rows.map(({ staffId: _staffId, ...r }) => r),
    });
  } catch (e) {
    console.error("[personnel] birthdates-import", e instanceof Error ? e.message : "erreur");
    return NextResponse.json({ error: "Erreur lors de l'import." }, { status: 500 });
  }
}
