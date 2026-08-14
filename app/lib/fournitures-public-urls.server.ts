import "server-only";

import {
  fournituresPublicFileApiUrl,
  isAllowedFournituresS3Key,
  resolveFournituresPublicHrefSync,
} from "@/app/lib/fournitures-public-urls";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";
import type { FournituresStage, FournituresToolConfig } from "@/app/lib/fournitures-types";

const STAGES: FournituresStage[] = ["ecole", "college", "lycee"];

/** Transforme chemins S3 / URLs bucket privé en route publique. */
async function resolveFournituresPublicHref(href: string): Promise<string> {
  const synced = resolveFournituresPublicHrefSync(href);
  const trimmed = href.trim();
  if (!trimmed || synced !== trimmed) return synced;

  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    const parsedKey = await parseTravelsS3KeyFromUrl(trimmed);
    if (parsedKey && isAllowedFournituresS3Key(parsedKey)) {
      return fournituresPublicFileApiUrl(parsedKey);
    }
  }

  return trimmed;
}

export async function resolveFournituresConfigPublicUrls(
  config: FournituresToolConfig,
): Promise<FournituresToolConfig> {
  const stageLinks = { ...config.stageLinks };
  for (const stage of STAGES) {
    const link = stageLinks[stage];
    if (!link?.url) continue;
    stageLinks[stage] = {
      ...link,
      url: await resolveFournituresPublicHref(link.url),
    };
  }
  return { ...config, stageLinks };
}
