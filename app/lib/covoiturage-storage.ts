import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  COVOITURAGE_S3,
  type CovoiturageMatch,
  type CovoiturageProfile,
} from "@/app/lib/covoiturage-types";

export async function getCovoiturageProfiles(): Promise<CovoiturageProfile[]> {
  const hit = await getJson<CovoiturageProfile[]>(COVOITURAGE_S3.profiles);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveCovoiturageProfiles(profiles: CovoiturageProfile[]) {
  await putJson(COVOITURAGE_S3.profiles, profiles);
}

export async function getCovoiturageMatches(): Promise<CovoiturageMatch[]> {
  const hit = await getJson<CovoiturageMatch[]>(COVOITURAGE_S3.matches);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveCovoiturageMatches(matches: CovoiturageMatch[]) {
  await putJson(COVOITURAGE_S3.matches, matches);
}

export async function getProfileByUserId(userId: string): Promise<CovoiturageProfile | null> {
  const profiles = await getCovoiturageProfiles();
  return profiles.find((p) => p.externalUserId === userId && p.status !== "unregistered") ?? null;
}
