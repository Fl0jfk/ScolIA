-- Contexte requête sur ouverture document dossier élève (RGPD / traçabilité).
ALTER TABLE "eleve_access_audit" ADD COLUMN IF NOT EXISTS "actor_ip" text;
--> statement-breakpoint
ALTER TABLE "eleve_access_audit" ADD COLUMN IF NOT EXISTS "actor_user_agent" text;
--> statement-breakpoint
ALTER TABLE "eleve_access_audit" ADD COLUMN IF NOT EXISTS "actor_roles" jsonb;
