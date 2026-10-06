/**
 * Règles statiques pour *.test.* — pas de DATABASE_URL, pas de chargement .env, pas de postgres() direct hors garde.
 */
const forbiddenDatabaseUrl =
  /process\.env\.(?:DATABASE_URL|SCOLIA_PROD_DATABASE_URL)|process\.env\[['"](?:DATABASE_URL|SCOLIA_PROD_DATABASE_URL)['"]\]|const\s*\{[^}]*\b(?:DATABASE_URL|SCOLIA_PROD_DATABASE_URL)\b[^}]*\}\s*=\s*process\.env/;

const envLoadPatterns = [
  { rule: "dotenv", re: /\bdotenv\b/ },
  { rule: ".env.local", re: /\.env\.local/ },
  { rule: "loadEnvFile", re: /\bloadEnvFile\b/ },
  { rule: "--env-file", re: /--env-file\b/ },
];

const directPostgresPatterns = [
  { rule: "postgres(", re: /\bpostgres\s*\(/ },
  { rule: "new Pool(", re: /\bnew\s+Pool\s*\(/ },
  { rule: "new Client(", re: /\bnew\s+Client\s*\(/ },
];

export const allowTestFiles = new Set([
  "app/lib/test-database-harness.ts",
  "scripts/test-database-guard.mjs",
  "scripts/test-database-guard.test.mjs",
]);

const guardImportPatterns = [
  /from\s+['"][^'"]*test-database-guard\.mjs['"]/,
  /from\s+['"][^'"]*test-database-harness['"]/,
  /require\s*\(\s*['"][^'"]*test-database-guard\.mjs['"]\s*\)/,
  /require\s*\(\s*['"][^'"]*test-database-harness['"]\s*\)/,
];

function isCommentOnlyLine(line) {
  const trimmed = line.trim();
  return !trimmed || trimmed.startsWith("//") || trimmed.startsWith("*");
}

export function lineReadsDatabaseUrl(line) {
  if (isCommentOnlyLine(line)) return false;
  if (/delete\s+process\.env\.(?:DATABASE_URL|SCOLIA_PROD_DATABASE_URL)/.test(line)) {
    return false;
  }
  if (/process\.env\.(?:DATABASE_URL|SCOLIA_PROD_DATABASE_URL)\s*=(?!=)/.test(line)) {
    return false;
  }
  return forbiddenDatabaseUrl.test(line);
}

export function contentImportsTestDatabaseGuard(content) {
  return guardImportPatterns.some((re) => re.test(content));
}

/** @deprecated utilise contentImportsTestDatabaseGuard */
export function usesTestDatabaseGuard(content) {
  return contentImportsTestDatabaseGuard(content);
}

/**
 * @param {string} relPath chemin relatif repo
 * @param {string} content
 * @returns {string[]} codes de violation (ex. "DATABASE_URL", "postgres(")
 */
export function scanTestFileContent(relPath, content) {
  if (allowTestFiles.has(relPath)) return [];

  const violations = new Set();
  const lines = content.split("\n");

  for (const line of lines) {
    if (lineReadsDatabaseUrl(line)) violations.add("DATABASE_URL");
    if (!isCommentOnlyLine(line)) {
      for (const { rule, re } of envLoadPatterns) {
        if (re.test(line)) violations.add(rule);
      }
    }
  }

  const hasGuard = contentImportsTestDatabaseGuard(content);
  if (!hasGuard) {
    for (const { rule, re } of directPostgresPatterns) {
      if (re.test(content)) violations.add(rule);
    }
  }

  return [...violations];
}

/**
 * @param {Record<string, string>} scripts section scripts de package.json
 * @returns {string[]} noms de scripts en violation
 */
export function scanPackageJsonTestScripts(scripts) {
  const bad = [];
  for (const [name, command] of Object.entries(scripts)) {
    if (!name.startsWith("test:")) continue;
    if (/\bdotenv\b/.test(command) || /--env-file\b/.test(command)) {
      bad.push(name);
    }
  }
  return bad;
}
