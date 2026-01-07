# Test Coverage Gaps

Production code that lacks adequate test coverage.

## 🔴 Complex Functions

### `src/view/index.rs`
- **`render_index_template(title: &str) -> Markup`** (29 lines)
  - **Complexity**: Generates entire HTML page structure with head/body/scripts
  - **Risk**: Critical function that renders the main application page
  - **Coverage**: Zero test coverage
  - **Recommendation**: Unit test HTML structure and title interpolation

### `src/view/score/linescore.rs`
- **`render_line_score_tables(bettors: &[BettorData], refresh_data: &RefreshData) -> Markup`** (121 lines)
  - **Complexity**: Very high - nested loops, conditional HTML generation, complex button/table rendering
  - **Risk**: Most complex view function with round selection logic, data transformation
  - **Coverage**: No direct test coverage
  - **Recommendation**: Unit tests for different bettor configurations, round states, empty data

### `src/view/score/scoreboard.rs`
- **`render_scoreboard(data: &ScoreData) -> Markup`** (69 lines)
  - **Complexity**: Moderate - HTML table generation with score formatting
  - **Risk**: Core scoreboard display functionality
  - **Coverage**: No direct test coverage
  - **Recommendation**: Unit test table structure and score display formatting

### `src/view/score/summary.rs`
- **`render_summary_scores(grouped_data: &AllBettorScoresByRound) -> Markup`** (36 lines)
  - **Complexity**: Moderate - summary table HTML generation
  - **Risk**: Summary view display
  - **Coverage**: No direct test coverage
  - **Recommendation**: Unit test summary table structure and data presentation

### `src/view/score/utils.rs`
- **`scores_and_last_refresh_to_line_score_tables(scores: &ScoresAndLastRefresh) -> Vec<BettorData>`** (~25 lines)
  - **Complexity**: Moderate - data transformation function
  - **Risk**: Data structure conversion for line score display
  - **Coverage**: No direct test coverage
  - **Recommendation**: Unit test data transformation logic with various input scenarios

## 🟡 Partially Tested Complex Functions

### `src/view/score/template.rs`
- **`render_scores_template(...) -> Result<Markup, Box<dyn std::error::Error>>`** (44 lines)
  - **Complexity**: High - orchestrates multiple sub-renders, async database calls
  - **Coverage**: ✅ Tested indirectly via test6 and test7 (full integration)
  - **Gap**: No direct unit testing of individual components or error paths
  - **Recommendation**: Unit tests for error handling and component integration

### `src/view/score/chart.rs`
- **`preprocess_golfer_data(...) -> Result<BTreeMap<String, Vec<GolferBars>>, Box<dyn std::error::Error>>`** (110+ lines)
  - **Complexity**: Very high - mathematical calculations, step factor logic, bar positioning
  - **Coverage**: ✅ Mentioned in test6 but not directly unit tested
  - **Gap**: Complex step factor calculations and scaling logic not isolated
  - **Recommendation**: Unit tests for step factor calculations, bar width computations, edge cases

- **`render_drop_down_bar(...) -> Result<Markup, Box<dyn std::error::Error>>`** (60+ lines)
  - **Complexity**: High - complex HTML generation with dynamic styling
  - **Coverage**: ✅ Tested indirectly via test06/test07 integration tests
  - **Gap**: No direct testing of HTML structure generation
  - **Recommendation**: Unit test HTML structure and CSS class generation

## Testing Priority Recommendations

### High Priority (Critical Business Logic)
1. **`render_line_score_tables()`** - Most complex untested function
2. **`preprocess_golfer_data()`** - Complex mathematical calculations 
3. **`render_index_template()`** - Critical main page render

### Medium Priority (Core Functionality)
4. **`render_scoreboard()`** - Core scoreboard display
5. **`scores_and_last_refresh_to_line_score_tables()`** - Data transformation
6. **`render_summary_scores()`** - Summary display

### Low Priority (Already Partially Covered)
7. **`render_scores_template()`** - Error path testing
8. **`render_drop_down_bar()`** - Direct HTML structure testing

## Test Strategy Suggestions

- **Unit Tests**: Focus on individual function behavior, edge cases, error conditions
- **Integration Tests**: Test component interactions and data flow
- **HTML Structure Tests**: Verify generated HTML structure and CSS classes
- **Data Transformation Tests**: Validate correct data processing and formatting
- **Error Handling Tests**: Test database failures, invalid data scenarios
