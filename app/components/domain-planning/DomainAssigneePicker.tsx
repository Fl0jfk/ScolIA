export type DirectoryAssigneeOption = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  pending?: boolean;
  /** Rôles intranet (pour filtre professeurs / personnels). */
  roles?: string[];
};
