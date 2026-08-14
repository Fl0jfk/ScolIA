import type { Establishment } from "@/app/lib/app-config-schemas";
import type { CovoiturageMatch, CovoiturageProfile } from "@/app/lib/covoiturage-types";

function matchPairKey(a: string, b: string) {
  return [a, b].sort().join("::");
}

function sharedZones(a: CovoiturageProfile, b: CovoiturageProfile): string[] {
  const setB = new Set(b.zones);
  return a.zones.filter((z) => setB.has(z));
}

function sharedEstablishments(a: CovoiturageProfile, b: CovoiturageProfile): string[] {
  const setB = new Set(b.establishments);
  return a.establishments.filter((e) => setB.has(e));
}

function existingPairMatch(
  matches: CovoiturageMatch[],
  userA: string,
  userB: string,
): CovoiturageMatch | undefined {
  const key = matchPairKey(userA, userB);
  return matches.find(
    (m) =>
      matchPairKey(m.profileA, m.profileB) === key &&
      (m.status === "pending" || m.status === "revealed"),
  );
}

export function findNewMatchesForProfile(
  profile: CovoiturageProfile,
  allProfiles: CovoiturageProfile[],
  existingMatches: CovoiturageMatch[],
): CovoiturageMatch[] {
  if (profile.status !== "active") return [];

  const created: CovoiturageMatch[] = [];
  const now = new Date().toISOString();

  for (const other of allProfiles) {
    if (other.clerkUserId === profile.clerkUserId) continue;
    if (other.status !== "active") continue;
    if (existingPairMatch(existingMatches, profile.clerkUserId, other.clerkUserId)) continue;
    if (existingPairMatch(created, profile.clerkUserId, other.clerkUserId)) continue;

    const zones = sharedZones(profile, other);
    const establishments = sharedEstablishments(profile, other);
    if (zones.length === 0 || establishments.length === 0) continue;

    created.push({
      id: `match_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      profileA: profile.clerkUserId,
      profileB: other.clerkUserId,
      matchedZone: zones[0]!,
      matchedEstablishments: establishments,
      status: "pending",
      acceptedA: false,
      acceptedB: false,
      createdAt: now,
    });
  }

  return created;
}

export function cancelPendingMatchesForUser(
  matches: CovoiturageMatch[],
  userId: string,
): CovoiturageMatch[] {
  return matches.map((m) => {
    if (m.status !== "pending") return m;
    if (m.profileA !== userId && m.profileB !== userId) return m;
    return { ...m, status: "cancelled" as const };
  });
}

export function establishmentLabels(ids: string[], establishments: Establishment[]): string {
  return ids
    .map((id) => establishments.find((e) => e.id === id)?.label ?? id)
    .join(", ");
}

export function otherPartyId(match: CovoiturageMatch, userId: string): string {
  return match.profileA === userId ? match.profileB : match.profileA;
}

function userAcceptedMatch(match: CovoiturageMatch, userId: string): boolean {
  if (match.profileA === userId) return match.acceptedA;
  if (match.profileB === userId) return match.acceptedB;
  return false;
}

export function applyMatchAcceptance(
  match: CovoiturageMatch,
  userId: string,
): CovoiturageMatch {
  const updated = { ...match };
  if (match.profileA === userId) updated.acceptedA = true;
  else if (match.profileB === userId) updated.acceptedB = true;

  if (updated.acceptedA && updated.acceptedB) {
    updated.status = "revealed";
    updated.revealedAt = new Date().toISOString();
  }
  return updated;
}

export function applyMatchDecline(match: CovoiturageMatch, userId: string): CovoiturageMatch {
  return {
    ...match,
    status: "declined",
    declinedBy: userId,
  };
}
