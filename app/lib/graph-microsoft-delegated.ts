import { getTenant } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import {
  microsoftRefreshTokenParams,
  postMicrosoftOAuthToken,
} from "@/app/lib/microsoft-oauth-token";

const GRAPH_SCOPES = "https://graph.microsoft.com/Files.ReadWrite offline_access User.Read";

type MicrosoftRefreshTokenResult =
  | { accessToken: string; refreshToken?: string }
  | { error: string };

/** Échange un refresh token Microsoft contre un access token Graph (dépôt OneDrive serveur). */
export async function getMicrosoftAccessTokenFromRefresh(
  refreshToken: string,
): Promise<MicrosoftRefreshTokenResult> {
  const token = refreshToken.trim();
  if (!token) return { error: "Refresh token vide." };

  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const ms = secrets?.microsoft;
  if (!ms?.tenantId || !ms?.clientId) {
    return { error: "Microsoft non configuré pour ce tenant." };
  }

  const result = await postMicrosoftOAuthToken(
    ms.tenantId,
    microsoftRefreshTokenParams({
      clientId: ms.clientId,
      refreshToken: token,
      scope: GRAPH_SCOPES,
      clientSecret: ms.clientSecret?.trim() || undefined,
    }),
  );

  if (!result.ok) {
    return { error: `Refresh token Microsoft : ${result.body.slice(0, 200)}` };
  }
  return {
    accessToken: result.tokens.accessToken,
    refreshToken: result.tokens.refreshToken,
  };
}
