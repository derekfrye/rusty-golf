use anyhow::Result;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;

pub(crate) enum ReplPromptError {
    Interrupted,
    Failed(anyhow::Error),
}

pub(crate) fn prompt_for_events(
    rl: &mut Editor<super::helper::ReplHelper, DefaultHistory>,
) -> Result<Vec<String>, ReplPromptError> {
    match rl.readline("Which events? (csv or space-separated) ") {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Ok(Vec::new());
            }
            let normalized = trimmed.replace(',', " ");
            let ids = normalized
                .split_whitespace()
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect();
            Ok(ids)
        }
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => Err(ReplPromptError::Interrupted),
        Err(err) => Err(ReplPromptError::Failed(anyhow::Error::from(err))),
    }
}
