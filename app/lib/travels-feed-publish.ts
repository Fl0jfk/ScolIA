/**
 * Publication de photos « tout ou rien » (testable : les accès stockage et base sont injectés).
 * Objets déposés d'abord ; la publication n'est créée qu'ensuite ; si l'insertion des photos échoue, la publication
 * est SUPPRIMÉE (jamais de publication « photos » vide visible) et les objets déposés sont retirés.
 */
export type StoredPhoto = { id: string; storageKey: string; width: number; height: number; bytes: number; position: number };

export type PhotoPostDeps = {
  putObject(path: string, bytes: Buffer): Promise<string>;
  insertPost(): Promise<{ id: string }>;
  insertPhotos(postId: string, photos: StoredPhoto[]): Promise<void>;
  deletePost(postId: string): Promise<void>;
  deleteObject(key: string): Promise<void>;
  newId(): string;
};

export async function storePhotosPost(
  deps: PhotoPostDeps,
  opts: { pathPrefix: string; cleaned: { bytes: Buffer; width: number; height: number }[] },
): Promise<{ postId: string; stored: StoredPhoto[] }> {
  const stored: StoredPhoto[] = [];
  let postId: string | null = null;
  try {
    for (let i = 0; i < opts.cleaned.length; i++) {
      const id = deps.newId();
      const key = await deps.putObject(`${opts.pathPrefix}/${id}.jpg`, opts.cleaned[i].bytes);
      stored.push({ id, storageKey: key, width: opts.cleaned[i].width, height: opts.cleaned[i].height, bytes: opts.cleaned[i].bytes.length, position: i });
    }
    const post = await deps.insertPost();
    postId = post.id;
    await deps.insertPhotos(post.id, stored);
    return { postId: post.id, stored };
  } catch (e) {
    if (postId) await deps.deletePost(postId).catch(() => undefined);
    await Promise.all(stored.map((s) => deps.deleteObject(s.storageKey).catch(() => undefined)));
    throw e;
  }
}
