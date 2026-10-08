/** Postgres `undefined_column` (ex. migration 0051 non appliquée en prod). */
export function isPgUndefinedColumnError(error: unknown, column?: string): boolean {
  const unwrap = (e: unknown): unknown => {
    if (e && typeof e === "object" && "cause" in e) {
      return (e as { cause?: unknown }).cause ?? e;
    }
    return e;
  };
  const cause = unwrap(error);
  if (!cause || typeof cause !== "object") return false;
  const code = (cause as { code?: string }).code;
  if (code !== "42703") return false;
  if (!column) return true;
  const message = String((cause as { message?: string }).message || "");
  return message.includes(column);
}
