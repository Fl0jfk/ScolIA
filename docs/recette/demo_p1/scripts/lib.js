const { chromium } = require('playwright');
const l = require('./labo.json');
const BASE = 'http://127.0.0.1:8090';
exports.l = l; exports.BASE = BASE;
exports.persona = (role, match) => l.personas.find(p => p.role === role && (!match || p.label.includes(match))).email;
exports.ouvrir = async (opts = {}) => {
  const b = await chromium.launch();
  const ctx = await b.newContext({ viewport: opts.viewport || { width: 1280, height: 800 }, deviceScaleFactor: opts.dsf || 2 });
  const p = await ctx.newPage();
  p.on('console', m => { if (m.type()==='error') console.log('  [console.error]', m.text().slice(0,250)); });
  p.on('pageerror', e => console.log('  [pageerror]', String(e).slice(0,250)));
  return { b, ctx, p };
};
exports.connecter = async (p, email) => {
  await p.goto(`${BASE}/web/connexion?etab=${l.etablissement_id}`, { waitUntil: 'networkidle' });
  await p.waitForTimeout(2500);
  await p.getByLabel('Adresse e-mail').fill(email);
  await p.getByLabel(/Mot de passe/).first().fill(l.password);
  await p.getByRole('button', { name: 'Continuer' }).click();
  await p.waitForURL(u => !u.pathname.includes('connexion'), { timeout: 15000 });
  await p.waitForTimeout(1500);
};
