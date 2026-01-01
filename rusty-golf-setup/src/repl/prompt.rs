use crate::repl::parse::split_items;
use anyhow::Result;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;

pub(crate) enum ReplPromptError {
    Interrupted,
    Failed(anyhow::Error),
}

pub(crate) fn prompt_for_items(
    rl: &mut Editor<super::helper::ReplHelper, DefaultHistory>,
    prompt: &str,
) -> Result<Vec<String>, ReplPromptError> {
    match rl.readline(prompt) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Ok(Vec::new());
            }
            Ok(split_items(trimmed))
        }
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => Err(ReplPromptError::Interrupted),
        Err(err) => Err(ReplPromptError::Failed(anyhow::Error::from(err))),
    }
}
