export type SitePostStatus = "draft" | "published";

export type SitePost = {
  id: string;
  slug: string;
  title: string;
  excerpt?: string;
  body: string;
  coverUrl?: string;
  status: SitePostStatus;
  createdAt: string;
  updatedAt: string;
  publishedAt?: string;
};

export type SitePostIndexEntry = {
  id: string;
  slug: string;
  title: string;
  excerpt?: string;
  status: SitePostStatus;
  updatedAt: string;
  publishedAt?: string;
};
