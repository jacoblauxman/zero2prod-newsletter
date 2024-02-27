-- Add migration script here
-- wrap whole migration in transaction to make sure it succeeds / fails automatically
-- note: `sqlx' does not do automatically
BEGIN;
  -- Backfill `status` for "historical" entries
  UPDATE subscriptions
  SET status = 'confirmed'
  WHERE status IS NULL;
  -- Then make `status` mandatory
  ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
