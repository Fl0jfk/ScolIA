/** Erreurs liées à une mauvaise config auth (clés / secret manquants). */
export function isAuthConfigError(error: unknown): boolean {
  const msg = error instanceof Error ? error.message : String(error);
  const code =
    error && typeof error === "object" && "code" in error
      ? String((error as { code?: string }).code ?? "")
      : "";
  return (
    code === "AUTH_SERVER_CONFIG" ||
    /BETTER_AUTH_SECRET/i.test(msg) ||
    /DATABASE_URL/i.test(msg) ||
    /configuration auth/i.test(msg)
  );
}

export const AUTH_CONFIG_HINT =
  "Vérifiez DATABASE_URL, BETTER_AUTH_SECRET et AUTH_PROVIDER=better-auth dans l’environnement.";
