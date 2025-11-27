-- Add the tracking columns
ALTER TABLE links
    ADD COLUMN click_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN last_used TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Add indexes for performance
CREATE INDEX idx_links_click_count ON links(click_count DESC);
CREATE INDEX idx_links_last_used ON links(last_used DESC);

-- Create the batch update function
CREATE OR REPLACE FUNCTION batch_update_clicks(
    link_ids TEXT[],  -- Changed from TEXT[]
    increments INTEGER[],
    timestamps TIMESTAMPTZ[]
) RETURNS INTEGER AS $$
DECLARE
    rows_updated INTEGER;
BEGIN
    -- Validate array lengths match
    IF array_length(link_ids, 1) != array_length(increments, 1) OR
       array_length(link_ids, 1) != array_length(timestamps, 1) THEN
        RAISE EXCEPTION 'Array length mismatch';
    END IF;

    -- Validate no negative increments
    IF EXISTS (SELECT 1 FROM unnest(increments) AS i WHERE i < 0) THEN
        RAISE EXCEPTION 'Negative increments not allowed';
    END IF;

    -- Perform the batch update
    UPDATE links AS l
    SET
        click_count = l.click_count + v.inc,
        last_used = GREATEST(l.last_used, v.ts)
    FROM (
        SELECT
            unnest(link_ids) AS link_id,
            unnest(increments) AS inc,
            unnest(timestamps) AS ts
    ) AS v
    WHERE l.id = v.link_id;

    GET DIAGNOSTICS rows_updated = ROW_COUNT;
    RETURN rows_updated;
END;
$$ LANGUAGE plpgsql;

-- Add comment for documentation
COMMENT ON FUNCTION batch_update_clicks IS
'Atomically updates click counts and last_used timestamps for multiple links. Safe for concurrent usage across multiple service instances.';

COMMENT ON COLUMN links.click_count IS
'Total number of times this link has been accessed';

COMMENT ON COLUMN links.last_used IS
'Timestamp of the most recent access to this link';
