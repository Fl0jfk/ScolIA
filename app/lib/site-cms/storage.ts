import "server-only";

import { getJson, putJson, deleteObject } from "@/app/lib/s3-storage";
import type { SitePost, SitePostIndexEntry } from "@/app/lib/site-cms/types";

const INDEX_KEY = "site-cms/posts/index.json";

export function postKey(id: string) {
  return `site-cms/posts/${id}.json`;
}

export function slugify(input: string): string {
  return String(input || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80) || `post-${Date.now().toString(36)}`;
}

export async function loadPostsIndex(): Promise<SitePostIndexEntry[]> {
  const hit = await getJson<SitePostIndexEntry[]>(INDEX_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function savePostsIndex(entries: SitePostIndexEntry[]) {
  await putJson(INDEX_KEY, entries);
}

export async function loadPost(id: string): Promise<SitePost | null> {
  const hit = await getJson<SitePost>(postKey(id));
  return hit?.data || null;
}

export async function savePost(post: SitePost): Promise<SitePost> {
  await putJson(postKey(post.id), post);
  const index = await loadPostsIndex();
  const entry: SitePostIndexEntry = {
    id: post.id,
    slug: post.slug,
    title: post.title,
    excerpt: post.excerpt,
    status: post.status,
    updatedAt: post.updatedAt,
    publishedAt: post.publishedAt,
  };
  await savePostsIndex([entry, ...index.filter((e) => e.id !== post.id)]);
  return post;
}

export async function deletePost(id: string): Promise<void> {
  await deleteObject(postKey(id));
  const index = await loadPostsIndex();
  await savePostsIndex(index.filter((e) => e.id !== id));
}

export async function listPublishedPosts(): Promise<SitePost[]> {
  const index = await loadPostsIndex();
  const published = index.filter((e) => e.status === "published");
  const out: SitePost[] = [];
  for (const e of published) {
    const full = await loadPost(e.id);
    if (full && full.status === "published") out.push(full);
  }
  return out.sort((a, b) =>
    String(b.publishedAt || b.updatedAt).localeCompare(String(a.publishedAt || a.updatedAt)),
  );
}
