-- Create x402 table with composite primary key
CREATE TABLE "public"."x402" (
    "network" VARCHAR NOT NULL,
    "tx_hash" VARCHAR NOT NULL,
    "link_id" VARCHAR NOT NULL,
    PRIMARY KEY ("network", "tx_hash"),
    FOREIGN KEY ("link_id") REFERENCES "public"."links"("id") ON DELETE RESTRICT
);

-- Create index on link_id for efficient lookups of x402 entries by link
CREATE INDEX "idx_x402_id" ON "public"."x402"("link_id");

-- Create view that shows links with their associated x402 entries
CREATE VIEW "public"."links_with_x402" AS
SELECT
    l.id,
    l.url,
    l.created_at,
    l.key,
    l.click_count,
    l.last_used,
    x.network,
    x.tx_hash
FROM "public"."links" l
LEFT JOIN "public"."x402" x ON l.id = x.link_id;
