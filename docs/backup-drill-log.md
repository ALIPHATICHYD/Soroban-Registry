# Disaster recovery drill log

Appended to by `scripts/disaster-recovery-drill.sh`. Each entry records one
rehearsal of recovering the registry database from the off-site repository.

Add a row by hand for real incidents too: an unplanned recovery is the most
honest drill there is.

| Date (UTC) | Result | RTO | RPO | Notes |
| --- | --- | --- | --- | --- |
| 2026-08-24T08:54:45Z | PASS | 0m | 2m | targets met |
| 2026-08-24T08:57:31Z | PASS | 0m | 0m | targets met |
| 2026-08-24T09:42:32Z | FAIL | 0m | 0m | restore from the off-site repository failed;  |
| 2026-08-24T09:44:34Z | FAIL | 0m | 1m | restore from the off-site repository failed;  |
| 2026-08-24T09:44:46Z | FAIL | 0m | 1m | restore from the off-site repository failed;  |
| 2026-08-24T09:45:22Z | PASS | 4m | 2m | targets met |
| 2026-08-24T09:55:17Z | FAIL | 0m | 0m | restore from the off-site repository failed;  |
| 2026-08-24T09:56:48Z | PASS | 0m | 2m | targets met |
| 2026-08-24T10:11:29Z | PASS | 0m | 0m | targets met |
| 2026-08-24T10:19:26Z | PASS | 0m | 0m | targets met |
