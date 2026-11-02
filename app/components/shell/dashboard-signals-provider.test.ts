import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  DEFAULT_DASHBOARD_SIGNALS_CONTEXT,
  useDashboardSignalsContext,
} from "@/app/components/shell/DashboardSignalsProvider";

test("DEFAULT_DASHBOARD_SIGNALS_CONTEXT — loading, refresh no-op", async () => {
  assert.equal(DEFAULT_DASHBOARD_SIGNALS_CONTEXT.loading, true);
  assert.equal(DEFAULT_DASHBOARD_SIGNALS_CONTEXT.notifications.length, 0);
  assert.equal(await DEFAULT_DASHBOARD_SIGNALS_CONTEXT.refresh(), null);
});

test("useDashboardSignalsContext — sans Provider actif ne throw pas", () => {
  function Probe() {
    const ctx = useDashboardSignalsContext();
    return React.createElement("span", {
      "data-loading": ctx.loading ? "1" : "0",
      "data-gen": String(ctx.fetchGeneration),
    });
  }
  const html = renderToStaticMarkup(React.createElement(Probe));
  assert.match(html, /data-loading="1"/);
  assert.match(html, /data-gen="0"/);
});
