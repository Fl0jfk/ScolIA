/** Forme minimale d’un utilisateur (client ou serveur). */
export type SessionLikeUser = {
  id?: string | null;
  fullName?: string | null;
  firstName?: string | null;
  lastName?: string | null;
  publicMetadata?: Record<string, unknown> | null;
  primaryEmailAddress?: { emailAddress?: string } | null;
  emailAddresses?: Array<{ emailAddress?: string }> | null;
};

export type AppActor = SessionLikeUser;
