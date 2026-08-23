/** Stub `server-only` / `client-only` pour scripts Node (tsx). */
const Module = require("node:module");
const original = Module.prototype.require;
Module.prototype.require = function patchedRequire(id) {
  if (id === "server-only" || id === "client-only") return {};
  return original.apply(this, arguments);
};
