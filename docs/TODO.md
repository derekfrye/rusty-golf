# TODO

## Score view regressions introduced in 9a21400 (resolved)
- `Score by Player` even-par rounds disappeared because `preprocess_golfer_data_pure` skipped `score == 0`; fixed by rendering a centered zero-width bar in `66d7116`.
- Golfer order inside each player block sorted by total score instead of `short_name`; restored alphabetical ordering in `66d7116`.
- Chart markup/class names (`drop-down-bar-chart`, `.bar-text`, legacy container structure) were restored for CSS compatibility in `66d7116` (with bettor-level ordering tweaks in `1f28c0c`).
- `Score by Golfer` tables now rebuild via `BTreeMap` for deterministic ordering and merged tee/line data, addressing the HashMap regression in `66d7116`.
- Stroke cells dropped the `score-shape-*` spans in favor of glyph-only classes; `6a55019` reinstated the legacy classes (without glyph noise) so CSS works again.
- Totals row reverted to “Total:” with per-round relative-to-par deltas instead of “Total Rn” raw strokes in `66d7116`.

## Reproduction notes
- Compared commit `9a21400` (“split up another large file”) against parent `9e92839`.
- Rendered HTML for `event=401580351&yr=2024` by mimicking the old async path: inspected `src/view/score.rs.bak` and new `chart.rs`/`linescore.rs`.
- Used the `tests/test03_espn_json_responses.json` fixture to seed both pipelines and diff the generated markup (ad-hoc Python helper; now removed from tree).
- Verified findings by tracing helper functions (`short_golfer_name`, `scores_and_last_refresh_to_line_score_tables`, etc.) and matching DOM snippets to CSS expectations.***
