#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandId {
    Help,
    ListEvents,
    GetAvailableGolfers,
    PickBettors,
    SetGolfersByBettor,
    SetupEvent,
    Exit,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubcommandId {
    Help,
    Refresh,
}

pub(crate) struct ReplCommand {
    pub(crate) id: CommandId,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) subcommands: &'static [ReplSubcommand],
}

pub(crate) struct ReplSubcommand {
    pub(crate) id: SubcommandId,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
}

const LIST_EVENTS_SUBCOMMANDS: &[ReplSubcommand] = &[
    ReplSubcommand {
        id: SubcommandId::Help,
        name: "help",
        description: "display this help screen",
    },
    ReplSubcommand {
        id: SubcommandId::Refresh,
        name: "refresh",
        description: "if passed, hit espn api again to refresh current events.",
    },
];

pub(crate) const REPL_COMMANDS: &[ReplCommand] = &[
    ReplCommand {
        id: CommandId::Help,
        name: "help",
        description: "Show this help.",
        aliases: &["?", "-h", "--help"],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::ListEvents,
        name: "list_events",
        description: "List events on ESPN API.",
        aliases: &[],
        subcommands: LIST_EVENTS_SUBCOMMANDS,
    },
    ReplCommand {
        id: CommandId::GetAvailableGolfers,
        name: "get_available_golfers",
        description: "Prompt for event IDs to use for golfers.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::PickBettors,
        name: "pick_bettors",
        description: "Prompt for bettor names.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::SetGolfersByBettor,
        name: "set_golfers_by_bettor",
        description: "Prompt for golfers for each bettor.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::SetupEvent,
        name: "setup_event",
        description: "Guide setup and write a new EUP event JSON.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::Exit,
        name: "exit",
        description: "Exit the REPL.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::Quit,
        name: "quit",
        description: "Exit the REPL.",
        aliases: &[],
        subcommands: &[],
    },
];

pub(crate) fn find_command(name: &str) -> Option<&'static ReplCommand> {
    REPL_COMMANDS
        .iter()
        .find(|command| command.name == name || command.aliases.contains(&name))
}

pub(crate) fn find_subcommand(
    subcommands: &'static [ReplSubcommand],
    name: &str,
) -> Option<&'static ReplSubcommand> {
    subcommands
        .iter()
        .find(|subcommand| subcommand.name == name)
}

pub(crate) fn print_subcommand_help(command: &ReplCommand) {
    for subcommand in command.subcommands {
        println!("{} {}", subcommand.name, subcommand.description);
    }
}

pub(crate) fn build_repl_help() -> String {
    let mut help = String::from("Commands:");
    for command in REPL_COMMANDS {
        let names = if command.aliases.is_empty() {
            command.name.to_string()
        } else {
            let mut parts = Vec::with_capacity(command.aliases.len() + 1);
            parts.push(command.name);
            parts.extend(command.aliases);
            parts.join(", ")
        };
        help.push_str("\n  ");
        help.push_str(&names);
        let padding = 22usize.saturating_sub(names.len());
        help.push_str(&" ".repeat(padding.max(2)));
        help.push_str(command.description);
    }
    help
}
