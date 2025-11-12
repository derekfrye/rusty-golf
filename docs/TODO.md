# TODO

## Score view regressions introduced in 9a21400
- `Score by Player` no longer renders even-par rounds after the async → pure refactor. `preprocess_golfer_data_pure` skips `score == 0`, so neutral rounds (e.g., Rory McIlroy R2/R4 for `event=401580351&yr=2024`) disappear.
- Golfer order inside each player block now sorts by total score instead of name; the switch from `short_name` sorting to `total_score` changes layout expectations.
- Markup/class names for the chart DOM changed (`drop-down-bar-chart` → `player-bar-container`, removal of `.bar-text`, etc.), breaking existing CSS.
- `Score by Golfer` table data now comes straight from a `HashMap`, so golfer tables no longer appear in stable alphabetical order and line/tee data isn’t merged as before.
- Stroke cells use new glyphs/classes (`▲/◆/●` + `birdie`, `bogey`, etc.) instead of the old `score-shape-*` spans, so styling regressed.
- Totals row switched from “Total:” per round-delta to “Total Rn” with raw stroke sums, producing different copy and numbers.

## Reproduction notes
- Compared commit `9a21400` (“split up another large file”) against parent `9e92839`.
- Rendered HTML for `event=401580351&yr=2024` by mimicking the old async path: inspected `src/view/score.rs.bak` and new `chart.rs`/`linescore.rs`.
- Used the `tests/test3_espn_json_responses.json` fixture to seed both pipelines and diff the generated markup (ad-hoc Python helper; now removed from tree).
- Verified findings by tracing helper functions (`short_golfer_name`, `scores_and_last_refresh_to_line_score_tables`, etc.) and matching DOM snippets to CSS expectations.***
