import "server-only";

import { deleteTeamsChatLink, loadTeamsChatLink, saveTeamsChatLink } from "@/app/lib/teams-chat/tokens";
import { refreshTeamsChatAccessToken } from "@/app/lib/teams-chat/oauth";
import type {
  TeamsChatMessage,
  TeamsChatPerson,
  TeamsChatSummary,
} from "@/app/lib/teams-chat/types";

const GRAPH = "https://graph.microsoft.com/v1.0";

type GraphMe = {
  id?: string;
  displayName?: string;
  mail?: string;
  userPrincipalName?: string;
};

type GraphPerson = {
  displayName?: string;
  userPrincipalName?: string;
  jobTitle?: string;
  scoredEmailAddresses?: Array<{ address?: string }>;
  personType?: { class?: string; subclass?: string };
};

type GraphChatMember = {
  userId?: string;
  displayName?: string;
  email?: string;
};

type GraphChat = {
  id?: string;
  chatType?: string;
  members?: GraphChatMember[];
  lastMessagePreview?: {
    createdDateTime?: string;
    body?: { content?: string };
  };
};

type GraphMessage = {
  id?: string;
  createdDateTime?: string;
  from?: { user?: { id?: string; displayName?: string } };
  body?: { contentType?: string; content?: string };
};

export class TeamsChatUnlinkedError extends Error {
  constructor(message = "Compte Microsoft non lié.") {
    super(message);
    this.name = "TeamsChatUnlinkedError";
  }
}

function htmlToText(html: string): string {
  return html
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<\/p>/gi, "\n")
    .replace(/<[^>]+>/g, "")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .trim();
}

async function graphJson<T>(accessToken: string, path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${GRAPH}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${accessToken}`,
      Accept: "application/json",
      ...(init?.body ? { "Content-Type": "application/json" } : {}),
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(`Graph ${res.status} : ${err.slice(0, 400)}`);
  }
  if (res.status === 204) return {} as T;
  return (await res.json()) as T;
}

export async function fetchTeamsGraphMe(accessToken: string): Promise<GraphMe> {
  return graphJson<GraphMe>(
    accessToken,
    "/me?$select=id,displayName,mail,userPrincipalName",
  );
}

export async function getTeamsChatAccessContext(
  clerkUserId: string,
  graphAccessToken?: string,
): Promise<{
  accessToken: string;
  microsoftUserId: string;
  displayName?: string;
  upn?: string;
}> {
  const headerToken = graphAccessToken?.trim() || "";
  if (headerToken) {
    const me = await fetchTeamsGraphMe(headerToken);
    if (!me.id) throw new TeamsChatUnlinkedError("Profil Microsoft sans id.");
    const link = await loadTeamsChatLink(clerkUserId);
    if (link?.microsoftUserId && link.microsoftUserId !== me.id) {
      throw new TeamsChatUnlinkedError(
        "Le compte Microsoft ne correspond pas au compte déjà lié.",
      );
    }
    return {
      accessToken: headerToken,
      microsoftUserId: me.id,
      displayName: me.displayName,
      upn: me.userPrincipalName || me.mail,
    };
  }

  const link = await loadTeamsChatLink(clerkUserId);
  if (!link) throw new TeamsChatUnlinkedError();

  try {
    const tokens = await refreshTeamsChatAccessToken(link.refreshToken);
    if (tokens.refreshToken && tokens.refreshToken !== link.refreshToken) {
      await saveTeamsChatLink({ ...link, refreshToken: tokens.refreshToken });
    }
    return {
      accessToken: tokens.accessToken,
      microsoftUserId: link.microsoftUserId,
      displayName: link.displayName,
      upn: link.upn,
    };
  } catch (e) {
    const msg = e instanceof Error ? e.message : "";
    if (/invalid_grant|AADSTS70000|AADSTS50173/i.test(msg)) {
      await deleteTeamsChatLink(clerkUserId).catch(() => undefined);
      throw new TeamsChatUnlinkedError("Session Microsoft expirée — veuillez relier votre compte.");
    }
    throw e;
  }
}

export async function searchTeamsChatPeople(
  accessToken: string,
  query: string,
): Promise<TeamsChatPerson[]> {
  const q = query.trim().slice(0, 80);
  if (q.length < 2) return [];

  const paths = [
    `/me/people?$search=${encodeURIComponent(`"${q}"`)}&$top=20`,
    `/me/people?$search=${encodeURIComponent(q)}&$top=20`,
  ];

  let people: GraphPerson[] = [];
  for (const path of paths) {
    try {
      const data = await graphJson<{ value?: GraphPerson[] }>(accessToken, path);
      people = data.value ?? [];
      break;
    } catch {
      /* syntaxe $search selon le tenant */
    }
  }

  const seen = new Set<string>();
  const out: TeamsChatPerson[] = [];
  for (const p of people) {
    const subclass = p.personType?.subclass;
    if (subclass && subclass !== "OrganizationUser") continue;
    const upn = p.userPrincipalName?.trim() || p.scoredEmailAddresses?.[0]?.address?.trim();
    if (!upn || seen.has(upn.toLowerCase())) continue;
    seen.add(upn.toLowerCase());
    out.push({
      id: upn,
      displayName: p.displayName?.trim() || upn,
      mail: p.scoredEmailAddresses?.[0]?.address || upn,
      userPrincipalName: upn,
      jobTitle: p.jobTitle || undefined,
    });
  }
  return out;
}

export async function listTeamsOneOnOneChats(
  accessToken: string,
  myMicrosoftUserId: string,
): Promise<TeamsChatSummary[]> {
  let data: { value?: GraphChat[] };
  try {
    data = await graphJson<{ value?: GraphChat[] }>(
      accessToken,
      "/me/chats?$expand=members,lastMessagePreview&$top=50",
    );
  } catch {
    data = await graphJson<{ value?: GraphChat[] }>(
      accessToken,
      "/me/chats?$expand=members&$top=50",
    );
  }

  const chats: TeamsChatSummary[] = [];
  for (const chat of data.value ?? []) {
    if (!chat.id || chat.chatType !== "oneOnOne") continue;
    const other = (chat.members ?? []).find((m) => m.userId && m.userId !== myMicrosoftUserId);
    if (!other?.userId) continue;
    const preview = chat.lastMessagePreview?.body?.content
      ? htmlToText(chat.lastMessagePreview.body.content)
      : undefined;
    chats.push({
      id: chat.id,
      other: {
        id: other.userId,
        displayName: other.displayName?.trim() || other.email || "Collègue",
        mail: other.email || undefined,
      },
      lastPreview: preview?.slice(0, 160) || undefined,
      lastAt: chat.lastMessagePreview?.createdDateTime,
    });
  }

  chats.sort((a, b) => (b.lastAt || "").localeCompare(a.lastAt || ""));
  return chats;
}

export async function ensureOneOnOneChat(
  accessToken: string,
  myMicrosoftUserId: string,
  otherUserId: string,
): Promise<string> {
  const body = {
    chatType: "oneOnOne",
    members: [
      {
        "@odata.type": "#microsoft.graph.aadUserConversationMember",
        roles: ["owner"],
        "kevin.m@example.com": `${GRAPH}/users('${myMicrosoftUserId}')`,
      },
      {
        "@odata.type": "#microsoft.graph.aadUserConversationMember",
        roles: ["owner"],
        "kevin.m@example.com": `${GRAPH}/users('${otherUserId}')`,
      },
    ],
  };
  const created = await graphJson<{ id?: string }>(accessToken, "/chats", {
    method: "POST",
    body: JSON.stringify(body),
  });
  if (!created.id) throw new Error("Graph n’a pas renvoyé d’id de conversation.");
  return created.id;
}

export async function listChatMessages(
  accessToken: string,
  chatId: string,
  myMicrosoftUserId: string,
): Promise<TeamsChatMessage[]> {
  const data = await graphJson<{ value?: GraphMessage[] }>(
    accessToken,
    `/chats/${encodeURIComponent(chatId)}/messages?$top=40`,
  );

  const messages: TeamsChatMessage[] = [];
  for (const m of data.value ?? []) {
    if (!m.id) continue;
    const fromId = m.from?.user?.id;
    const raw = m.body?.content || "";
    const text = htmlToText(raw);
    if (!text && m.body?.contentType === "html") continue;
    messages.push({
      id: m.id,
      fromMe: Boolean(fromId && fromId === myMicrosoftUserId),
      fromName: m.from?.user?.displayName?.trim() || "Collègue",
      text: text || "(message)",
      createdAt: m.createdDateTime || new Date().toISOString(),
    });
  }

  messages.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  return messages;
}

export async function sendChatMessage(
  accessToken: string,
  chatId: string,
  text: string,
): Promise<void> {
  const content = text.trim();
  if (!content) throw new Error("Message vide.");
  await graphJson(accessToken, `/chats/${encodeURIComponent(chatId)}/messages`, {
    method: "POST",
    body: JSON.stringify({
      body: { contentType: "text", content: content.slice(0, 4000) },
    }),
  });
}

export async function fetchUserPhotoBytes(
  accessToken: string,
  userId: string,
): Promise<{ bytes: Buffer; contentType: string } | null> {
  const res = await fetch(`${GRAPH}/users/${encodeURIComponent(userId)}/photo/$value`, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (res.status === 404) return null;
  if (!res.ok) return null;
  const buf = Buffer.from(await res.arrayBuffer());
  if (!buf.length) return null;
  const contentType = res.headers.get("content-type") || "image/jpeg";
  return { bytes: buf, contentType };
}
