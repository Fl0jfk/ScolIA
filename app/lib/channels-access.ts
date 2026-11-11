/** Contrôles d'accès des salons du personnel (NJ-04, D-053 lot 2). Fonctions pures, testables. */

export type ChannelLike = {
  id?: string;
  type?: string;
  members?: unknown;
  roleRequired?: string;
  createdBy?: string;
};

export type ChannelViewer = { userId: string; roles: string[]; isAdmin: boolean };

function memberIds(c: ChannelLike): string[] {
  return Array.isArray(c.members) ? c.members.map(String) : [];
}

export function canReadChannel(c: ChannelLike, v: ChannelViewer): boolean {
  if (v.isAdmin) return true;
  if (c.type === "private") return memberIds(c).includes(v.userId);
  if (c.type === "restricted") return !!c.roleRequired && v.roles.includes(c.roleRequired);
  return true;
}

/** Modifier ou supprimer un salon : admin, créateur, ou membre d'un salon privé. Salon ancien sans créateur : admin seulement (sauf privé). */
export function canManageChannel(c: ChannelLike, v: ChannelViewer): boolean {
  if (v.isAdmin) return true;
  if (c.createdBy && c.createdBy === v.userId) return true;
  return c.type === "private" && memberIds(c).includes(v.userId);
}

/** Supprimer un message : son auteur ou un admin. */
export function canDeleteChannelMessage(m: { authorId?: unknown }, v: ChannelViewer): boolean {
  if (v.isAdmin) return true;
  return typeof m.authorId === "string" && m.authorId === v.userId;
}
