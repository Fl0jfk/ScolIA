/**
 * Messages OAuth OneDrive RH — notamment URI de redirection Azure manquante.
 *
 * RAPPEL CONFIG AZURE (App Registration → Authentification → URI de redirection Web) :
 *   https://{votre-domaine}/api/rh/drive/oauth/callback
 *   https://{votre-domaine}/api/teams-chat/oauth/callback
 * Exemple local (si test MSAL local) : http://localhost:3000/api/rh/drive/oauth/callback
 */

export const RH_AZURE_REDIRECT_URI_HINT =
  "Ajoutez l'URI Web « /api/rh/drive/oauth/callback » dans Azure (App Registration → Authentification → URI de redirection, plateforme Web).";

const REDIRECT_URI_ERROR_PATTERNS = [
  /AADSTS50011/i,
  /redirect_uri/i,
  /reply address/i,
  /reply url/i,
  /redirect URI/i,
  /invalid_redirect/i,
  /does not match the redirect/i,
  /reply address registered/i,
];

export function isRhOAuthRedirectUriError(raw: string): boolean {
  const s = raw.trim();
  if (!s) return false;
  return REDIRECT_URI_ERROR_PATTERNS.some((re) => re.test(s));
}

export function formatRhOAuthError(raw: string, redirectUri?: string | null): string {
  const detail = raw.trim();
  if (!detail) return RH_AZURE_REDIRECT_URI_HINT;

  if (isRhOAuthRedirectUriError(detail)) {
    const uriLine = redirectUri ? `\nURI attendue : ${redirectUri}` : "";
    return `L'URI de redirection n'est probablement pas enregistrée dans Microsoft Azure.\n${RH_AZURE_REDIRECT_URI_HINT}${uriLine}\n\nDétail Microsoft : ${detail.slice(0, 220)}`;
  }

  if (/clientSecret|client_secret|invalid_client/i.test(detail)) {
    return `Secret client Microsoft manquant ou invalide pour ce tenant. Vérifiez la configuration plateforme.\n\nDétail : ${detail.slice(0, 220)}`;
  }

  return detail.slice(0, 280);
}

/** Code query `rhDrive=azure_redirect` pour l'UI. */
export const RH_OAUTH_ERROR_CODE_AZURE_REDIRECT = "azure_redirect";

export function rhOAuthErrorQueryParam(raw: string): string {
  if (isRhOAuthRedirectUriError(raw)) return RH_OAUTH_ERROR_CODE_AZURE_REDIRECT;
  return "error";
}
