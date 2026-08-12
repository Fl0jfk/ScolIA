/**
 * Pages publiques visiteurs (rentrée, simulateurs, landing…) où l'assistant IA
 * et le lien « Espace personnel » ne doivent pas apparaître.
 */
export function isPublicVisitorPath(pathname: string | null | undefined): boolean {
  const path = (pathname ?? "").toLowerCase();
  if (!path || path === "/") return true;

  const exact = new Set([
    "/connexion",
    "/plateforme",
    "/mentions-legales",
    "/tarifs",
    "/demande/merci",
    "/demande-parents",
  ]);

  if (exact.has(path)) return true;

  const prefixes = [
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
  ];

  return prefixes.some((prefix) => path === prefix || path.startsWith(`${prefix}/`));
}
