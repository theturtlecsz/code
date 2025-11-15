-- Fix auto-vacuum (requires VACUUM to take effect)
-- SQLite auto_vacuum can only be set on empty DB or after VACUUM

.print "=== CURRENT AUTO-VACUUM SETTING ==="
PRAGMA auto_vacuum;

.print ""
.print "=== DATABASE SIZE BEFORE ==="
SELECT page_count * page_size / 1024 / 1024 AS size_mb FROM pragma_page_count(), pragma_page_size();

.print ""
.print "=== ENABLING INCREMENTAL AUTO-VACUUM (setting pragma) ==="
PRAGMA auto_vacuum = INCREMENTAL;

.print ""
.print "=== RUNNING VACUUM TO APPLY AUTO-VACUUM ==="
.print "(This will reclaim 153MB of space and apply auto-vacuum setting)"
VACUUM;

.print ""
.print "=== VERIFYING AUTO-VACUUM ENABLED ==="
PRAGMA auto_vacuum;

.print ""
.print "=== DATABASE SIZE AFTER ==="
SELECT page_count * page_size / 1024 / 1024 AS size_mb FROM pragma_page_count(), pragma_page_size();

.print ""
.print "=== FREELIST SIZE (should be minimal after VACUUM) ==="
SELECT freelist_count * page_size / 1024 / 1024 AS freelist_mb FROM pragma_freelist_count(), pragma_page_size();

.print ""
.print "Auto-vacuum migration complete!"
