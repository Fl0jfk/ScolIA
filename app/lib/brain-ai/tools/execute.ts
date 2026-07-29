import { assertToolPermissions } from "@/app/lib/brain-ai/permissions";
import { getBrainTool } from "@/app/lib/brain-ai/tools/registry";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

function sanitizeArgs(args: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(args)) {
    if (/pass|token|secret|authorization/i.test(k)) {
      out[k] = "[redacted]";
      continue;
    }
    if (typeof v === "string" && v.length > 500) out[k] = `${v.slice(0, 500)}…`;
    else out[k] = v;
  }
  return out;
}

export async function executeBrainTool(
  name: string,
  rawArgs: unknown,
  ctx: BrainToolCtx,
): Promise<BrainToolResult> {
  const tool = getBrainTool(name);
  if (!tool) {
    return { ok: false, error: `Outil inconnu: ${name}`, code: "UNKNOWN_TOOL" };
  }

  const gate = assertToolPermissions(ctx, tool);
  if (!gate.ok) {
    console.info("[brain-ai] tool denied", {
      userId: ctx.userId,
      tool: name,
      code: gate.code,
    });
    return { ok: false, error: gate.error, code: gate.code };
  }

  const args =
    rawArgs && typeof rawArgs === "object" && !Array.isArray(rawArgs)
      ? (rawArgs as Record<string, unknown>)
      : {};

  try {
    const result = await tool.handler(ctx, args);
    const needsConfirm =
      !result.ok && "needsConfirmation" in result && result.needsConfirmation === true;
    console.info("[brain-ai] tool", {
      userId: ctx.userId,
      tool: name,
      args: sanitizeArgs(args),
      ok: result.ok,
      needsConfirm,
      error: result.ok ? undefined : "error" in result ? result.error : undefined,
    });
    return result;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("[brain-ai] tool error", { userId: ctx.userId, tool: name, error: message });
    return { ok: false, error: message, code: "TOOL_ERROR" };
  }
}
