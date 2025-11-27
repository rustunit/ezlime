-- Drop the function
DROP FUNCTION IF EXISTS batch_update_clicks(TEXT[], INTEGER[], TIMESTAMPTZ[]);

-- Drop the indexes
DROP INDEX IF EXISTS idx_links_last_used;
DROP INDEX IF EXISTS idx_links_click_count;

-- Drop the columns
ALTER TABLE links
    DROP COLUMN IF EXISTS last_used,
    DROP COLUMN IF EXISTS click_count;
