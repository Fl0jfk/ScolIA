/** Erreurs Clerk liées aux clés dynamiques multi-tenant (proxy + CLERK_ENCRYPTION_KEY). */
export function isClerkDynamicKeyError(error: unknown): boolean {
  const msg = error instanceof Error ? error.message : String(error);
  const code =
    error && typeof error === "object" && "code" in error
      ? String((error as { code?: string }).code ?? "")
      : "";
  return (
    code === "encryption_key_invalid" ||
    /unable to decrypt request data/i.test(msg) ||
    /encryption key is invalid/i.test(msg) ||
    /CLERK_ENCRYPTION_KEY/i.test(msg)
  );
}

export const CLERK_ENCRYPTION_KEY_HINT =
  "Clerk multi-tenant : définissez CLERK_ENCRYPTION_KEY dans l’environnement de production (32+ caractères aléatoires, stable entre déploiements).";
