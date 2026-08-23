export type TeamsChatLink = {
  externalUserId: string;
  refreshToken: string;
  microsoftUserId: string;
  upn?: string;
  displayName?: string;
  linkedAt: string;
};

export type TeamsChatPerson = {
  id: string;
  displayName: string;
  mail?: string;
  userPrincipalName?: string;
  jobTitle?: string;
};

export type TeamsChatSummary = {
  id: string;
  other: TeamsChatPerson;
  lastPreview?: string;
  lastAt?: string;
};

export type TeamsChatMessage = {
  id: string;
  fromMe: boolean;
  fromName: string;
  text: string;
  createdAt: string;
};

export type TeamsChatStatus = {
  enabled: boolean;
  allowed: boolean;
  linked: boolean;
  me?: { displayName: string; upn?: string };
  oauthRedirectUri?: string | null;
  error?: string | null;
};
