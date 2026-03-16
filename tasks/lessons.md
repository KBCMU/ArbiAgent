# Lessons Learned

Format for each entry: `[date] | what went wrong | rule to prevent it`

---

2026-03-15 | Rust cargo build fails on OneDrive paths ("output path is not a writable directory") | Set `$env:CARGO_TARGET_DIR = "C:\tmp\arbiagent-target"` before running cargo commands
2026-03-15 | Polymarket events missing game_date because Gamma API date fields (endDate, startDate) were not being parsed | Always check what fields the API returns; Polymarket titles rarely contain dates but API response does
2026-03-15 | CBB/CFB matching failed because team dictionary had zero college entries, relying on "fuzzy matching" that didn't work | College teams need explicit dictionary entries for major programs — abbreviation-only matching is too brittle
2026-03-15 | NHL Utah Hockey Club was mapped to "ARI" (Arizona Coyotes, now defunct) | Keep team dictionaries current with franchise relocations/renames
2026-03-15 | Polymarket fetcher limit=100 cut off events | Paginate API calls with offset parameter; sports can have 100+ active events across leagues
2026-03-15 | Polymarket sports events often use grouped markets (one per team with Yes/No outcomes) instead of a single moneyline market with team names | Always check for grouped structure: scan ALL markets, use groupItemTitle as team label for Yes tokens
2026-03-15 | Label resolver only scanned one market for token→label mapping, missing tokens from grouped events | Build token→label mapping across ALL markets, not just the first/moneyline one
2026-03-15 | Discovery loop skipped cache updates when labels were already good, but token_ids could be stale | Compare token_ids between discovered and cached events; force update + clear odds when they differ
