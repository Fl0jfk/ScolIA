CREATE TABLE "tenant_document" (
	"etablissement_id" uuid NOT NULL,
	"doc_key" text NOT NULL,
	"payload" jsonb NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "tenant_document_pk" PRIMARY KEY("etablissement_id","doc_key")
);--> statement-breakpoint
ALTER TABLE "tenant_document" ADD CONSTRAINT "tenant_document_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "tenant_document_etablissement_idx" ON "tenant_document" USING btree ("etablissement_id");
