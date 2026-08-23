/**
 * Catalogue des chemins publics.
 * - PROXY_PUBLIC_ROUTE_MATCHERS : pas d’auth (proxy).
 * - isPublicVisitorPath : pages visiteurs (masquer assistant IA / espace personnel).
 * Les deux listes ne sont pas interchangeables : une API cron peut être
 * publique auth sans être une page visiteur.
 */

/** Matchers proxy `createRouteMatcher` (proxy). */
export const PROXY_PUBLIC_ROUTE_MATCHERS = [
  "/",
  "/rentree(.*)",
  "/documents/rentree(.*)",
  "/simulateurTarifs(.*)",
  "/simulateurFournitures(.*)",
  "/portes-ouvertes(.*)",
  "/repartition-classes(.*)",
  "/api/toolbox/public",
  "/api/toolbox/class-allocation/public(.*)",
  "/api/rentree/file",
  "/api/rentree/submissions(.*)",
  "/api/fournitures/file",
  "/api/portes-ouvertes/register",
  "/faire-une-demande(.*)",
  "/demande-parents(.*)",
  "/demande/merci",
  "/onboarding-rh(.*)",
  "/api/rh/onboarding/public(.*)",
  "/api/agentIAOCR/batch-job/internal-run",
  "/api/travels/ingest-from-email",
  "/api/travels/poll-email",
  "/api/requests/create",
  "/api/requests/confirm",
  "/api/requests/parent-portal",
  "/api/supplies/send",
  "/api/supplies/pdf",
  "/api/chatbot",
  "/api/site/public",
  "/api/public/site/posts",
  "/api/tenant/public",
  "/api/tenants/public",
  "/connexion",
  "/plateforme",
  "/sign-in(.*)",
  "/sign-up(.*)",
  "/auth(.*)",
  "/api/auth(.*)",
  "/api/auth/status",
  "/sign-out(.*)",
  "/mentions-legales",
  "/tarifs",
  "/internat/autorisation(.*)",
  "/internat/installation(.*)",
  "/api/internat/outings/decision(.*)",
  "/api/internat/installation/public",
  "/api/internat/installation/register",
  "/api/internat/installation/confirm",
  "/stages/eleve(.*)",
  "/stages/deposer(.*)",
  "/stages/signer(.*)",
  "/stages/candidater(.*)",
  "/api/stages/public(.*)",
  "/certificates/verify(.*)",
  "/api/certificates/verify(.*)",
  "/souscrire(.*)",
  "/api/platform/signup-requests",
  "/api/platform/signup-requests/status",
  "/api/billing/easytransac/checkout",
  "/api/billing/easytransac/webhook",
] as const;

const VISITOR_EXACT = new Set([
  "/connexion",
  "/plateforme",
  "/mentions-legales",
  "/tarifs",
  "/demande/merci",
  "/demande-parents",
]);

const VISITOR_PREFIXES = [
  "/souscrire",
  "/plateforme/demandes",
  "/plateforme/tenants",
  "/rentree",
  "/documents/rentree",
  "/simulateurtarifs",
  "/simulateurfournitures",
  "/portes-ouvertes",
  "/repartition-classes",
  "/faire-une-demande",
  "/demande-parents",
  "/internat/autorisation",
  "/internat/installation",
  "/stages/eleve",
  "/stages/deposer",
  "/stages/signer",
  "/stages/candidater",
  "/certificates/verify",
  "/api/toolbox/class-allocation/public",
  "/api/requests/parent-portal",
] as const;

/**
 * Pages publiques visiteurs (rentrée, simulateurs, landing…) où l'assistant IA
 * et le lien « Espace personnel » ne doivent pas apparaître.
 */
export function isPublicVisitorPath(pathname: string | null | undefined): boolean {
  const path = (pathname ?? "").toLowerCase();
  if (!path || path === "/") return true;
  if (VISITOR_EXACT.has(path)) return true;
  return VISITOR_PREFIXES.some((prefix) => path === prefix || path.startsWith(`${prefix}/`));
}
