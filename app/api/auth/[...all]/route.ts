import { toNextJsHandler } from "better-auth/next-js";
import { getBetterAuth } from "@/app/lib/auth-server";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";

const handler = isBetterAuthConfigured()
  ? toNextJsHandler(getBetterAuth())
  : {
      GET: async () =>
        new Response(JSON.stringify({ error: "Better-Auth non configuré." }), {
          status: 503,
          headers: { "content-type": "application/json" },
        }),
      POST: async () =>
        new Response(JSON.stringify({ error: "Better-Auth non configuré." }), {
          status: 503,
          headers: { "content-type": "application/json" },
        }),
    };

export const { GET, POST } = handler;
