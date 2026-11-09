export async function register() {
  // Job serveur des nouveautés de l'ENT (Web Push) : runtime Node uniquement, jamais au build ni en test.
  if (process.env.NEXT_RUNTIME !== "nodejs") return;
  if (process.env.NEXT_PHASE === "phase-production-build") return;
  if (process.env.SCOLIA_TEST_MODE === "1") return;
  try {
    const { startSignalsJob } = await import("./app/lib/web-push/signals-job");
    startSignalsJob();
  } catch (err) {
    console.error("[instrumentation] job signaux non démarré", err instanceof Error ? err.name : "error");
  }
}
