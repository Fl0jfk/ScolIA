ALTER TABLE "user" RENAME COLUMN "clerk_user_id" TO "external_user_id";--> statement-breakpoint
ALTER INDEX IF EXISTS "user_clerk_user_id_idx" RENAME TO "user_external_user_id_idx";--> statement-breakpoint
ALTER TABLE "clerk_user_mapping" RENAME TO "auth_user_mapping";--> statement-breakpoint
ALTER TABLE "auth_user_mapping" RENAME COLUMN "clerk_user_id" TO "external_user_id";--> statement-breakpoint
ALTER INDEX IF EXISTS "clerk_user_mapping_clerk_uidx" RENAME TO "auth_user_mapping_external_uidx";--> statement-breakpoint
ALTER INDEX IF EXISTS "clerk_user_mapping_user_uidx" RENAME TO "auth_user_mapping_user_uidx";
