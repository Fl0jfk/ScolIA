/** Forme minimale d’un user Clerk (client ou serveur). */
export type ClerkLikeUser = {
  id?: string | null;
  fullName?: string | null;
  firstName?: string | null;
  lastName?: string | null;
  publicMetadata?: Record<string, unknown> | null;
  primaryEmailAddress?: { emailAddress?: string } | null;
  emailAddresses?: Array<{ emailAddress?: string }> | null;
};

export type ClerkActor = ClerkLikeUser;
