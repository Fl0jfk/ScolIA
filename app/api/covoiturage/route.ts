import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { loadAppConfig } from "@/app/lib/app-config";
import {
  applyMatchAcceptance,
  applyMatchDecline,
  cancelPendingMatchesForUser,
  findNewMatchesForProfile,
  otherPartyId,
} from "@/app/lib/covoiturage-matching";
import {
  notifyCovoiturageContactRevealed,
  notifyCovoiturageMatchPotential,
} from "@/app/lib/covoiturage-notify";
import {
  getCovoiturageMatches,
  getCovoiturageProfiles,
  getProfileByUserId,
  saveCovoiturageMatches,
  saveCovoiturageProfiles,
} from "@/app/lib/covoiturage-storage";
import {
  currentSchoolYear,
  normalizePostalCode,
  type CovoiturageDirection,
  type CovoiturageProfile,
} from "@/app/lib/covoiturage-types";
import { requireAuth } from "@/app/lib/intranet-auth";

function displayName(user: NonNullable<Awaited<ReturnType<typeof safeCurrentUser>>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  const full = `${first} ${last}`.trim();
  if (full) return full;
  return user?.primaryEmailAddress?.emailAddress?.split("@")[0] || "Utilisateur";
}

function userEmail(user: NonNullable<Awaited<ReturnType<typeof safeCurrentUser>>>) {
  return user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
}

function parseZones(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  const out: string[] = [];
  for (const item of raw) {
    const z = normalizePostalCode(String(item ?? ""));
    if (z && !out.includes(z)) out.push(z);
  }
  return out;
}

function parseEstablishments(raw: unknown, validIds: string[]): string[] {
  if (!Array.isArray(raw)) return validIds.length === 1 ? [validIds[0]!] : [];
  const set = new Set(validIds);
  const out = raw.map(String).filter((id) => set.has(id));
  return [...new Set(out)];
}

function parseDirection(raw: unknown): CovoiturageDirection {
  if (raw === "morning" || raw === "evening" || raw === "both") return raw;
  return "both";
}

function sanitizeProfileForClient(
  profile: CovoiturageProfile | null,
  matches: Awaited<ReturnType<typeof getCovoiturageMatches>>,
  userId: string,
  allProfiles: CovoiturageProfile[],
) {
  const myMatches = matches
    .filter((m) => m.profileA === userId || m.profileB === userId)
    .filter((m) => m.status === "pending" || m.status === "revealed")
    .map((m) => {
      const otherId = otherPartyId(m, userId);
      const other = allProfiles.find((p) => p.externalUserId === otherId);
      const revealed = m.status === "revealed";
      return {
        id: m.id,
        status: m.status,
        matchedZone: m.matchedZone,
        matchedEstablishments: m.matchedEstablishments,
        createdAt: m.createdAt,
        revealedAt: m.revealedAt,
        myAccepted: m.profileA === userId ? m.acceptedA : m.acceptedB,
        otherAccepted: m.profileA === userId ? m.acceptedB : m.acceptedA,
        other:
          revealed && other
            ? { displayName: other.displayName, email: other.email }
            : revealed
              ? null
              : { displayName: "Famille inscrite", email: null },
      };
    });

  return { profile, matches: myMatches };
}

async function runMatchingAndNotify(profile: CovoiturageProfile) {
  const [profiles, matches] = await Promise.all([
    getCovoiturageProfiles(),
    getCovoiturageMatches(),
  ]);

  const newMatches = findNewMatchesForProfile(profile, profiles, matches);
  if (newMatches.length === 0) return;

  const allMatches = [...matches, ...newMatches];
  await saveCovoiturageMatches(allMatches);

  for (const match of newMatches) {
    const profileA = profiles.find((p) => p.externalUserId === match.profileA);
    const profileB = profiles.find((p) => p.externalUserId === match.profileB);
    if (profileA?.status === "active") {
      await notifyCovoiturageMatchPotential({ profile: profileA, match }).catch(() => {});
    }
    if (profileB?.status === "active") {
      await notifyCovoiturageMatchPotential({ profile: profileB, match }).catch(() => {});
    }
  }
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const bundle = await loadAppConfig();
  const establishments = bundle.establishments.filter((e) => e.active !== false);
  const [profile, matches, allProfiles] = await Promise.all([
    getProfileByUserId(userId),
    getCovoiturageMatches(),
    getCovoiturageProfiles(),
  ]);

  const payload = sanitizeProfileForClient(profile, matches, userId, allProfiles);

  return NextResponse.json({
    ...payload,
    establishments,
    schoolYear: currentSchoolYear(),
    disclaimer:
      "Mise en relation entre familles. Les arrangements de covoiturage relèvent de la responsabilité des parents. Aucune adresse exacte n'est stockée.",
  });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const user = await safeCurrentUser();
  if (!user) return NextResponse.json({ error: "Utilisateur introuvable." }, { status: 401 });

  const body = (await req.json()) as Record<string, unknown>;
  const action = String(body.action ?? "").trim();

  const bundle = await loadAppConfig();
  const validEstIds = bundle.establishments.filter((e) => e.active !== false).map((e) => e.id);

  if (action === "register" || action === "update") {
    const zones = parseZones(body.zones);
    if (zones.length === 0) {
      return NextResponse.json({ error: "Ajoutez au moins un code postal valide (5 chiffres)." }, { status: 400 });
    }

    const establishments = parseEstablishments(body.establishments, validEstIds);
    if (establishments.length === 0) {
      return NextResponse.json({ error: "Sélectionnez au moins un établissement." }, { status: 400 });
    }

    const direction = parseDirection(body.direction);
    const note = String(body.note ?? "").trim().slice(0, 280);
    const now = new Date().toISOString();
    const email = userEmail(user);
    if (!email) {
      return NextResponse.json({ error: "E-mail requis pour le covoiturage." }, { status: 400 });
    }

    const profiles = await getCovoiturageProfiles();
    const existingIdx = profiles.findIndex((p) => p.externalUserId === userId);

    const profile: CovoiturageProfile = {
      externalUserId: userId,
      displayName: displayName(user),
      email,
      status: "active",
      establishments,
      zones,
      direction,
      note: note || undefined,
      schoolYear: currentSchoolYear(),
      registeredAt: existingIdx >= 0 ? profiles[existingIdx]!.registeredAt : now,
      updatedAt: now,
      consentAt: existingIdx >= 0 ? profiles[existingIdx]!.consentAt : now,
    };

    if (existingIdx >= 0) profiles[existingIdx] = profile;
    else profiles.push(profile);

    await saveCovoiturageProfiles(profiles);
    await runMatchingAndNotify(profile);

    const matches = await getCovoiturageMatches();
    const allProfiles = await getCovoiturageProfiles();
    return NextResponse.json({
      ok: true,
      ...sanitizeProfileForClient(profile, matches, userId, allProfiles),
    });
  }

  if (action === "complete") {
    const profiles = await getCovoiturageProfiles();
    const idx = profiles.findIndex((p) => p.externalUserId === userId && p.status !== "unregistered");
    if (idx < 0) return NextResponse.json({ error: "Profil introuvable." }, { status: 404 });

    profiles[idx] = {
      ...profiles[idx]!,
      status: "complete",
      updatedAt: new Date().toISOString(),
    };
    await saveCovoiturageProfiles(profiles);

    const matches = cancelPendingMatchesForUser(await getCovoiturageMatches(), userId);
    await saveCovoiturageMatches(matches);

    const allProfiles = await getCovoiturageProfiles();
    return NextResponse.json({
      ok: true,
      ...sanitizeProfileForClient(profiles[idx]!, matches, userId, allProfiles),
    });
  }

  if (action === "reactivate") {
    const profiles = await getCovoiturageProfiles();
    const idx = profiles.findIndex((p) => p.externalUserId === userId && p.status === "complete");
    if (idx < 0) return NextResponse.json({ error: "Profil introuvable." }, { status: 404 });

    profiles[idx] = {
      ...profiles[idx]!,
      status: "active",
      updatedAt: new Date().toISOString(),
    };
    await saveCovoiturageProfiles(profiles);
    await runMatchingAndNotify(profiles[idx]!);

    const matches = await getCovoiturageMatches();
    const allProfiles = await getCovoiturageProfiles();
    return NextResponse.json({
      ok: true,
      ...sanitizeProfileForClient(profiles[idx]!, matches, userId, allProfiles),
    });
  }

  if (action === "unregister") {
    const profiles = await getCovoiturageProfiles();
    const idx = profiles.findIndex((p) => p.externalUserId === userId && p.status !== "unregistered");
    if (idx < 0) return NextResponse.json({ error: "Profil introuvable." }, { status: 404 });

    profiles[idx] = {
      ...profiles[idx]!,
      status: "unregistered",
      updatedAt: new Date().toISOString(),
    };
    await saveCovoiturageProfiles(profiles);

    const matches = cancelPendingMatchesForUser(await getCovoiturageMatches(), userId);
    await saveCovoiturageMatches(matches);

    return NextResponse.json({ ok: true, profile: null, matches: [] });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
