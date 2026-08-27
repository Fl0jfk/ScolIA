/**
 * Affiche le code TOTP courant pour le compte seed local.
 * Usage : npm run seed:dev:totp
 */
import { createOTP } from "@better-auth/utils/otp";
import { DEV_SEED } from "./seed-dev-local";

async function main() {
  const otp = createOTP(DEV_SEED.totpSecret, { period: 30, digits: 6 });
  const code = await otp.totp();
  const period = 30;
  const remaining = period - (Math.floor(Date.now() / 1000) % period);
  console.log(
    JSON.stringify(
      {
        email: DEV_SEED.email,
        code,
        validForSeconds: remaining,
        tip: "Utiliser sur /auth/sign-in après e-mail + mot de passe",
      },
      null,
      2,
    ),
  );
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
