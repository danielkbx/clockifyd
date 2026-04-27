#![allow(dead_code)]

pub const FORMAT_VALUES: &[&str] = &["text", "json", "raw"];
pub const ROUNDING_VALUES: &[&str] = &["off", "1m", "5m", "10m", "15m"];
pub const COMPLETION_SHELLS: &[&str] = &["bash", "zsh", "fish"];
pub const SKILL_SCOPE_VALUES: &[&str] = &["brief", "standard", "full"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: &'static str,
    pub about: &'static str,
    pub subcommands: Vec<CommandSpec>,
    pub options: Vec<OptionSpec>,
    pub positionals: Vec<PositionalSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSpec {
    pub long: Option<&'static str>,
    pub short: Option<char>,
    pub about: &'static str,
    pub value_name: Option<&'static str>,
    pub repeatable: bool,
    pub values: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionalSpec {
    pub name: &'static str,
    pub about: &'static str,
    pub repeatable: bool,
    pub values: &'static [&'static str],
}

impl CommandSpec {
    pub fn find(&self, path: &[&str]) -> Option<&CommandSpec> {
        match path.split_first() {
            None => Some(self),
            Some((name, rest)) => self
                .subcommands
                .iter()
                .find(|command| command.name == *name)
                .and_then(|command| command.find(rest)),
        }
    }

    pub fn command_paths(&self) -> Vec<Vec<&'static str>> {
        let mut paths = Vec::new();
        for command in &self.subcommands {
            collect_command_paths(command, Vec::new(), &mut paths);
        }
        paths
    }

    pub fn option_long_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        collect_option_long_names(self, &mut names);
        names.sort_unstable();
        names.dedup();
        names
    }
}

fn collect_command_paths(
    command: &CommandSpec,
    mut prefix: Vec<&'static str>,
    paths: &mut Vec<Vec<&'static str>>,
) {
    prefix.push(command.name);

    if command.subcommands.is_empty() {
        paths.push(prefix);
        return;
    }

    for subcommand in &command.subcommands {
        collect_command_paths(subcommand, prefix.clone(), paths);
    }
}

fn collect_option_long_names(command: &CommandSpec, names: &mut Vec<&'static str>) {
    names.extend(command.options.iter().filter_map(|option| option.long));

    for subcommand in &command.subcommands {
        collect_option_long_names(subcommand, names);
    }
}

pub fn cli_spec() -> CommandSpec {
    command(
        "cfd",
        "Clockify time tracking CLI",
        vec![
            option_value("format", "Output format", "format", FORMAT_VALUES),
            option_flag("no-meta", "Suppress metadata"),
            option_value("workspace", "Override configured workspace", "id", &[]),
            option_flag("no-rounding", "Disable configured rounding for one command"),
            option_short_flag('y', "Skip confirmation prompts"),
            option_flag("version", "Print version"),
        ],
        vec![],
        vec![
            command(
                "help",
                "Show help",
                vec![],
                vec![positional("command", "Command to show help for", true, &[])],
                vec![],
            ),
            leaf("login", "Store Clockify credentials"),
            leaf("logout", "Clear stored credentials"),
            command(
                "skill",
                "Print SKILL.md guidance",
                vec![
                    option_value("scope", "Skill detail level", "scope", SKILL_SCOPE_VALUES),
                    option_value("workspace", "Workspace ID", "id", &[]),
                    option_value("project", "Project ID", "id", &[]),
                ],
                vec![],
                vec![],
            ),
            leaf("whoami", "Show current Clockify user"),
            command(
                "workspace",
                "Manage workspaces",
                vec![],
                vec![],
                vec![
                    command(
                        "list",
                        "List workspaces",
                        vec![columns_option()],
                        vec![],
                        vec![],
                    ),
                    command(
                        "get",
                        "Show workspace",
                        vec![],
                        vec![positional("id", "Workspace ID", false, &[])],
                        vec![],
                    ),
                ],
            ),
            command(
                "config",
                "Manage stored defaults",
                vec![],
                vec![],
                vec![
                    leaf("interactive", "Interactively update stored defaults"),
                    config_key_command("set", "Set a stored default", true),
                    config_key_command("get", "Print a stored default", false),
                    config_key_command("unset", "Clear a stored default", false),
                ],
            ),
            command(
                "alias",
                "Manage local timer aliases",
                vec![],
                vec![],
                vec![
                    command(
                        "create",
                        "Create or update alias",
                        vec![
                            option_value("project", "Project ID", "id", &[]),
                            option_value("task", "Task ID or none", "id|none", &[]),
                            option_value(
                                "description",
                                "Entry description or none",
                                "text|none",
                                &[],
                            ),
                        ],
                        vec![positional("alias", "Alias name", false, &[])],
                        vec![],
                    ),
                    leaf("list", "List aliases"),
                    command(
                        "delete",
                        "Delete alias",
                        vec![],
                        vec![positional("alias", "Alias name", false, &[])],
                        vec![],
                    ),
                ],
            ),
            metadata_command("project", "Manage projects", "Project ID"),
            metadata_command("client", "Manage clients", "Client ID"),
            metadata_command("tag", "Manage tags", "Tag ID"),
            command(
                "task",
                "Manage tasks",
                vec![],
                vec![],
                vec![
                    command(
                        "list",
                        "List tasks",
                        vec![
                            option_value("project", "Project ID", "id", &[]),
                            columns_option(),
                        ],
                        vec![],
                        vec![],
                    ),
                    command(
                        "get",
                        "Show task",
                        vec![],
                        vec![
                            positional("project-id", "Project ID", false, &[]),
                            positional("task-id", "Task ID", false, &[]),
                        ],
                        vec![],
                    ),
                    command(
                        "create",
                        "Create task",
                        vec![
                            option_value("project", "Project ID", "id", &[]),
                            option_value("name", "Task name", "text", &[]),
                        ],
                        vec![],
                        vec![],
                    ),
                ],
            ),
            command(
                "entry",
                "Manage time entries",
                vec![],
                vec![],
                vec![
                    command(
                        "list",
                        "List time entries",
                        vec![
                            option_value("start", "Start filter", "iso|today|yesterday", &[]),
                            option_value("end", "End filter", "iso|today|yesterday", &[]),
                            option_value("project", "Project ID", "id", &[]),
                            option_value("task", "Task ID", "id", &[]),
                            option_value_repeatable("tag", "Tag ID", "id", &[]),
                            option_value("text", "Description text filter", "value", &[]),
                            columns_option(),
                        ],
                        vec![],
                        vec![],
                    ),
                    command(
                        "get",
                        "Show time entry",
                        vec![columns_option()],
                        vec![positional("id", "Time entry ID", false, &[])],
                        vec![],
                    ),
                    entry_mutation_command("add", "Create time entry", false),
                    entry_mutation_command("update", "Update time entry", true),
                    command(
                        "delete",
                        "Delete time entry",
                        vec![],
                        vec![positional("id", "Time entry ID", false, &[])],
                        vec![],
                    ),
                    command(
                        "text",
                        "Reuse prior entry descriptions",
                        vec![],
                        vec![],
                        vec![command(
                            "list",
                            "List known entry texts",
                            vec![
                                option_value("project", "Project ID", "id", &[]),
                                columns_option(),
                            ],
                            vec![],
                            vec![],
                        )],
                    ),
                ],
            ),
            leaf("today", "Show today's time entries"),
            command(
                "timer",
                "Manage running timer",
                vec![],
                vec![],
                vec![
                    leaf("current", "Show running timer"),
                    command(
                        "start",
                        "Start timer",
                        vec![
                            option_value("start", "Start timestamp", "iso", &[]),
                            option_value("project", "Project ID", "id", &[]),
                            option_value("task", "Task ID", "id", &[]),
                            option_value_repeatable("tag", "Tag ID", "id", &[]),
                        ],
                        vec![],
                        vec![],
                    ),
                    command(
                        "stop",
                        "Stop timer",
                        vec![option_value("end", "End timestamp", "iso", &[])],
                        vec![],
                        vec![],
                    ),
                    command(
                        "resume",
                        "Resume a recent time entry",
                        vec![
                            option_value("start", "Start timestamp", "iso", &[]),
                            option_short_flag('1', "Resume newest entry"),
                            option_short_flag('2', "Resume second newest entry"),
                            option_short_flag('3', "Resume third newest entry"),
                            option_short_flag('4', "Resume fourth newest entry"),
                            option_short_flag('5', "Resume fifth newest entry"),
                            option_short_flag('6', "Resume sixth newest entry"),
                            option_short_flag('7', "Resume seventh newest entry"),
                            option_short_flag('8', "Resume eighth newest entry"),
                            option_short_flag('9', "Resume ninth newest entry"),
                        ],
                        vec![],
                        vec![],
                    ),
                ],
            ),
            command(
                "completion",
                "Generate shell completions",
                vec![],
                vec![],
                COMPLETION_SHELLS
                    .iter()
                    .map(|shell| leaf(shell, "Generate completion script"))
                    .collect(),
            ),
        ],
    )
}

fn metadata_command(
    name: &'static str,
    about: &'static str,
    id_about: &'static str,
) -> CommandSpec {
    command(
        name,
        about,
        vec![],
        vec![],
        vec![
            command(
                "list",
                "List resources",
                vec![columns_option()],
                vec![],
                vec![],
            ),
            command(
                "get",
                "Show resource",
                vec![],
                vec![positional("id", id_about, false, &[])],
                vec![],
            ),
        ],
    )
}

fn config_key_command(name: &'static str, about: &'static str, needs_value: bool) -> CommandSpec {
    let mut subcommands = vec![
        config_value_command("workspace", "Workspace default", needs_value, &[]),
        config_value_command("project", "Project default", needs_value, &[]),
        config_value_command("rounding", "Rounding default", needs_value, ROUNDING_VALUES),
    ];

    if name != "set" {
        for command in &mut subcommands {
            command.positionals.clear();
        }
    }

    command(name, about, vec![], vec![], subcommands)
}

fn config_value_command(
    name: &'static str,
    about: &'static str,
    needs_value: bool,
    values: &'static [&'static str],
) -> CommandSpec {
    let positionals = if needs_value {
        vec![positional("value", "Config value", false, values)]
    } else {
        vec![]
    };

    command(name, about, vec![], positionals, vec![])
}

fn entry_mutation_command(name: &'static str, about: &'static str, has_id: bool) -> CommandSpec {
    let positionals = if has_id {
        vec![positional("id", "Time entry ID", false, &[])]
    } else {
        vec![]
    };

    command(
        name,
        about,
        vec![
            option_value("start", "Start timestamp", "iso", &[]),
            option_value("end", "End timestamp", "iso", &[]),
            option_value("duration", "Duration", "duration", &[]),
            option_value("project", "Project ID", "id", &[]),
            option_value("task", "Task ID", "id", &[]),
            option_value_repeatable("tag", "Tag ID", "id", &[]),
            option_value("description", "Entry description", "text", &[]),
        ],
        positionals,
        vec![],
    )
}

fn columns_option() -> OptionSpec {
    option_value("columns", "Columns to print", "list", &[])
}

fn command(
    name: &'static str,
    about: &'static str,
    options: Vec<OptionSpec>,
    positionals: Vec<PositionalSpec>,
    subcommands: Vec<CommandSpec>,
) -> CommandSpec {
    CommandSpec {
        name,
        about,
        subcommands,
        options,
        positionals,
    }
}

fn leaf(name: &'static str, about: &'static str) -> CommandSpec {
    command(name, about, vec![], vec![], vec![])
}

fn option_flag(long: &'static str, about: &'static str) -> OptionSpec {
    OptionSpec {
        long: Some(long),
        short: None,
        about,
        value_name: None,
        repeatable: false,
        values: &[],
    }
}

fn option_short_flag(short: char, about: &'static str) -> OptionSpec {
    OptionSpec {
        long: None,
        short: Some(short),
        about,
        value_name: None,
        repeatable: false,
        values: &[],
    }
}

fn option_value(
    long: &'static str,
    about: &'static str,
    value_name: &'static str,
    values: &'static [&'static str],
) -> OptionSpec {
    OptionSpec {
        long: Some(long),
        short: None,
        about,
        value_name: Some(value_name),
        repeatable: false,
        values,
    }
}

fn option_value_repeatable(
    long: &'static str,
    about: &'static str,
    value_name: &'static str,
    values: &'static [&'static str],
) -> OptionSpec {
    OptionSpec {
        repeatable: true,
        ..option_value(long, about, value_name, values)
    }
}

fn positional(
    name: &'static str,
    about: &'static str,
    repeatable: bool,
    values: &'static [&'static str],
) -> PositionalSpec {
    PositionalSpec {
        name,
        about,
        repeatable,
        values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_commands_exist() {
        let spec = cli_spec();
        let commands = spec
            .subcommands
            .iter()
            .map(|command| command.name)
            .collect::<Vec<_>>();

        assert_eq!(
            commands,
            vec![
                "help",
                "login",
                "logout",
                "skill",
                "whoami",
                "workspace",
                "config",
                "alias",
                "project",
                "client",
                "tag",
                "task",
                "entry",
                "today",
                "timer",
                "completion",
            ]
        );
    }

    #[test]
    fn completion_exposes_supported_shells() {
        let spec = cli_spec();
        let completion = spec.find(&["completion"]).unwrap();
        let shells = completion
            .subcommands
            .iter()
            .map(|command| command.name)
            .collect::<Vec<_>>();

        assert_eq!(shells, COMPLETION_SHELLS);
    }

    #[test]
    fn global_flags_include_expected_names() {
        let spec = cli_spec();
        let long_flags = spec
            .options
            .iter()
            .filter_map(|option| option.long)
            .collect::<Vec<_>>();
        let short_flags = spec
            .options
            .iter()
            .filter_map(|option| option.short)
            .collect::<Vec<_>>();

        assert!(long_flags.contains(&"format"));
        assert!(long_flags.contains(&"no-meta"));
        assert!(long_flags.contains(&"workspace"));
        assert!(long_flags.contains(&"no-rounding"));
        assert!(long_flags.contains(&"version"));
        assert!(short_flags.contains(&'y'));
    }

    #[test]
    fn format_values_match_constant() {
        let spec = cli_spec();
        let format = spec
            .options
            .iter()
            .find(|option| option.long == Some("format"))
            .unwrap();

        assert_eq!(format.values, FORMAT_VALUES);
    }

    #[test]
    fn config_set_rounding_exposes_rounding_values() {
        let spec = cli_spec();
        let rounding = spec
            .find(&["config", "set", "rounding"])
            .unwrap()
            .positionals
            .iter()
            .find(|positional| positional.name == "value")
            .unwrap();

        assert_eq!(rounding.values, ROUNDING_VALUES);
    }

    #[test]
    fn skill_scope_exposes_supported_values() {
        let spec = cli_spec();
        let scope = spec
            .find(&["skill"])
            .unwrap()
            .options
            .iter()
            .find(|option| option.long == Some("scope"))
            .unwrap();

        assert_eq!(scope.values, SKILL_SCOPE_VALUES);
    }

    #[test]
    fn command_paths_include_nested_leaves() {
        let spec = cli_spec();
        let paths = spec.command_paths();

        assert!(paths.contains(&vec!["login"]));
        assert!(paths.contains(&vec!["skill"]));
        assert!(paths.contains(&vec!["workspace", "list"]));
        assert!(paths.contains(&vec!["config", "set", "rounding"]));
        assert!(paths.contains(&vec!["entry", "text", "list"]));
        assert!(paths.contains(&vec!["today"]));
    }

    #[test]
    fn option_long_names_include_global_and_command_options() {
        let spec = cli_spec();
        let names = spec.option_long_names();

        assert!(names.contains(&"format"));
        assert!(names.contains(&"columns"));
        assert!(names.contains(&"duration"));
        assert!(names.contains(&"description"));
    }
}
