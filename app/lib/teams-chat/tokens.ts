import "server-only";

import { deleteObject, getJson, putJson } from "@/app/lib/s3-storage";
import type { TeamsChatLink } from "@/app/lib/teams-chat/types";

function linkPath(clerkUserId: string): string {
  return `teams-chat/links/${encodeURIComponent(clerkUserId)}.json`;
}

export async function loadTeamsChatLink(clerkUserId: string): Promise<TeamsChatLink | null> {
  const row = await getJson<TeamsChatLink>(linkPath(clerkUserId));
  const data = row?.data;
  if (!data?.refreshToken?.trim() || !data.microsoftUserId) return null;
  return data;
}

export async function saveTeamsChatLink(link: TeamsChatLink): Promise<void> {
  await putJson(linkPath(link.clerkUserId), link);
}

export async function deleteTeamsChatLink(clerkUserId: string): Promise<void> {
  await deleteObject(linkPath(clerkUserId));
}
