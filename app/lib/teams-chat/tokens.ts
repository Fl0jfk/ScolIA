import "server-only";

import { deleteObject, getJson, putJson } from "@/app/lib/s3-storage";
import type { TeamsChatLink } from "@/app/lib/teams-chat/types";

function linkPath(externalUserId: string): string {
  return `teams-chat/links/${encodeURIComponent(externalUserId)}.json`;
}

export async function loadTeamsChatLink(externalUserId: string): Promise<TeamsChatLink | null> {
  const row = await getJson<TeamsChatLink>(linkPath(externalUserId));
  const data = row?.data;
  if (!data?.refreshToken?.trim() || !data.microsoftUserId) return null;
  return data;
}

export async function saveTeamsChatLink(link: TeamsChatLink): Promise<void> {
  await putJson(linkPath(link.externalUserId), link);
}

export async function deleteTeamsChatLink(externalUserId: string): Promise<void> {
  await deleteObject(linkPath(externalUserId));
}
