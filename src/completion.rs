#![allow(dead_code)]

use crate::cli_spec::{CommandSpec, OptionSpec};
use crate::error::CfdError;

pub fn render_completion(shell: &str, spec: &CommandSpec) -> Result<String, CfdError> {
    match shell {
        "bash" => Ok(render_bash(spec)),
        "zsh" => Ok(render_zsh(spec)),
        "fish" => Ok(render_fish(spec)),
        _ => Err(CfdError::message(format!(
            "unsupported completion shell: {shell}"
        ))),
    }
}

fn render_bash(spec: &CommandSpec) -> String {
    let command_cases = bash_command_cases(spec);
    let all_options = words(
        collect_options(spec)
            .iter()
            .map(|option| option_name(option)),
    );
    let format_values = words(values_for_option(spec, "format").iter().copied());
    let rounding_values = words(
        values_for_positional(spec, &["config", "set", "rounding"], "value")
            .iter()
            .copied(),
    );

    format!(
        r#"# cfd bash completion
_cfd()
{{
    local cur prev candidates path
    local words=()
    COMPREPLY=()
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    case "$prev" in
        --format)
            COMPREPLY=( $(compgen -W "{format_values}" -- "$cur") )
            return 0
            ;;
        rounding)
            COMPREPLY=( $(compgen -W "{rounding_values}" -- "$cur") )
            return 0
            ;;
    esac

    case "$cur" in
        --*)
            COMPREPLY=( $(compgen -W "{all_options}" -- "$cur") )
            ;;
        *)
            for ((i = 1; i < COMP_CWORD; i++)); do
                case "${{COMP_WORDS[i]}}" in
                    --format|--workspace|--columns|--project|--start|--end|--text|--task|--tag|--duration|--description|--name)
                        ((i++))
                        ;;
                    --format=*|--workspace=*|--columns=*|--project=*|--start=*|--end=*|--text=*|--task=*|--tag=*|--duration=*|--description=*|--name=*)
                        ;;
                    -*)
                        ;;
                    *)
                        words+=("${{COMP_WORDS[i]}}")
                        ;;
                esac
            done

            path="${{words[*]}}"
            case "$path" in
{command_cases}
                *)
                    candidates=""
                    ;;
            esac

            COMPREPLY=( $(compgen -W "$candidates" -- "$cur") )
            ;;
    esac
}}

complete -F _cfd cfd
"#
    )
}

fn bash_command_cases(spec: &CommandSpec) -> String {
    let mut cases = Vec::new();
    cases.push(format!(
        "                \"\")\n                    candidates=\"{}\"\n                    ;;",
        words(spec.subcommands.iter().map(|command| command.name))
    ));

    for command in &spec.subcommands {
        collect_bash_command_cases(command, Vec::new(), &mut cases);
    }

    cases.join("\n")
}

fn collect_bash_command_cases(
    command: &CommandSpec,
    mut path: Vec<&'static str>,
    cases: &mut Vec<String>,
) {
    path.push(command.name);

    if !command.subcommands.is_empty() {
        cases.push(format!(
            "                \"{}\")\n                    candidates=\"{}\"\n                    ;;",
            path.join(" "),
            words(command.subcommands.iter().map(|command| command.name))
        ));

        for subcommand in &command.subcommands {
            collect_bash_command_cases(subcommand, path.clone(), cases);
        }
    }
}

fn render_zsh(spec: &CommandSpec) -> String {
    let options = words(
        collect_options(spec)
            .iter()
            .map(|option| option_name(option)),
    );
    let format_values = words(values_for_option(spec, "format"));
    let rounding_values = words(values_for_positional(
        spec,
        &["config", "set", "rounding"],
        "value",
    ));
    let command_cases = zsh_command_cases(spec);

    format!(
        r#"#compdef cfd

_cfd()
{{
  local -a commands
  local cur prev
  local path
  cur="${{words[CURRENT]}}"
  prev="${{words[CURRENT-1]}}"

  case "$prev" in
    --format)
      compadd -- {format_values}
      return
      ;;
    rounding)
      compadd -- {rounding_values}
      return
      ;;
  esac

  if [[ "$cur" == --* ]]; then
    compadd -- {options}
    return
  fi

  local -a path_words
  local i word
  for (( i = 2; i < CURRENT; i++ )); do
    word="${{words[i]}}"
    case "$word" in
      --format|--workspace|--columns|--project|--start|--end|--text|--task|--tag|--duration|--description|--name)
        (( i++ ))
        ;;
      --format=*|--workspace=*|--columns=*|--project=*|--start=*|--end=*|--text=*|--task=*|--tag=*|--duration=*|--description=*|--name=*)
        ;;
      -*)
        ;;
      *)
        path_words+=("$word")
        ;;
    esac
  done
  path="${{path_words[*]}}"

  case "$path" in
{command_cases}
    *)
      _message 'no more cfd subcommands'
      ;;
  esac
}}

_cfd "$@"
"#
    )
}

fn zsh_command_cases(spec: &CommandSpec) -> String {
    let mut cases = Vec::new();
    cases.push(format!(
        "        \"\")\n          commands=({})\n          _describe 'cfd command' commands\n          ;;",
        zsh_command_entries(&spec.subcommands)
    ));

    for command in &spec.subcommands {
        collect_zsh_command_cases(command, Vec::new(), &mut cases);
    }

    cases.join("\n")
}

fn collect_zsh_command_cases(
    command: &CommandSpec,
    mut path: Vec<&'static str>,
    cases: &mut Vec<String>,
) {
    path.push(command.name);

    if !command.subcommands.is_empty() {
        cases.push(format!(
            "        \"{}\")\n          commands=({})\n          _describe 'cfd subcommand' commands\n          ;;",
            path.join(" "),
            zsh_command_entries(&command.subcommands)
        ));

        for subcommand in &command.subcommands {
            collect_zsh_command_cases(subcommand, path.clone(), cases);
        }
    }
}

fn zsh_command_entries(commands: &[CommandSpec]) -> String {
    commands
        .iter()
        .map(|command| {
            format!(
                "'{}:{}'",
                zsh_escape(command.name),
                zsh_escape(command.about)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_fish(spec: &CommandSpec) -> String {
    let mut lines = vec!["# cfd fish completion".to_string()];

    for option in collect_options(spec) {
        lines.push(fish_option_line(option));
    }

    let top_level_commands = command_names(&spec.subcommands);
    for command in &spec.subcommands {
        render_fish_command_lines(command, &[], &top_level_commands, &mut lines);
    }

    lines.push(format!(
        "complete -c cfd -f -n '__fish_seen_argument --format' -a '{}'",
        words(values_for_option(spec, "format"))
    ));
    lines.push(format!(
        "complete -c cfd -f -n '__fish_seen_subcommand_from config; and __fish_seen_subcommand_from set; and __fish_seen_subcommand_from rounding' -a '{}'",
        words(values_for_positional(
            spec,
            &["config", "set", "rounding"],
            "value"
        ))
    ));

    lines.push(String::new());
    lines.join("\n")
}

fn render_fish_command_lines(
    command: &CommandSpec,
    parents: &[&str],
    siblings: &[&str],
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "complete -c cfd -f -n '{}' -a '{}' -d '{}'",
        fish_command_condition(parents, siblings),
        fish_escape(command.name),
        fish_escape(command.about)
    ));

    let mut child_parents = parents.to_vec();
    child_parents.push(command.name);
    let child_siblings = command_names(&command.subcommands);

    for subcommand in &command.subcommands {
        render_fish_command_lines(subcommand, &child_parents, &child_siblings, lines);
    }
}

fn collect_options(spec: &CommandSpec) -> Vec<&OptionSpec> {
    let mut options = Vec::new();
    collect_options_recursive(spec, &mut options);
    options.sort_by_key(|option| (option.long, option.short));
    options.dedup_by_key(|option| (option.long, option.short));
    options
}

fn command_words(spec: &CommandSpec) -> Vec<&'static str> {
    let mut names = Vec::new();
    collect_command_words(spec, &mut names);
    names.sort_unstable();
    names.dedup();
    names
}

fn collect_command_words(command: &CommandSpec, names: &mut Vec<&'static str>) {
    for subcommand in &command.subcommands {
        names.push(subcommand.name);
        collect_command_words(subcommand, names);
    }
}

fn collect_options_recursive<'a>(command: &'a CommandSpec, options: &mut Vec<&'a OptionSpec>) {
    options.extend(command.options.iter());

    for subcommand in &command.subcommands {
        collect_options_recursive(subcommand, options);
    }
}

fn values_for_option(spec: &CommandSpec, long: &str) -> &'static [&'static str] {
    collect_options(spec)
        .into_iter()
        .find(|option| option.long == Some(long))
        .map(|option| option.values)
        .unwrap_or(&[])
}

fn values_for_positional(spec: &CommandSpec, path: &[&str], name: &str) -> &'static [&'static str] {
    spec.find(path)
        .and_then(|command| {
            command
                .positionals
                .iter()
                .find(|positional| positional.name == name)
        })
        .map(|positional| positional.values)
        .unwrap_or(&[])
}

fn option_name(option: &OptionSpec) -> String {
    match (option.long, option.short) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => String::new(),
    }
}

fn zsh_option_argument(option: &&OptionSpec) -> String {
    match (option.long, option.short, option.value_name) {
        (Some(long), _, Some(value_name)) if !option.values.is_empty() => format!(
            "'--{}[{}]:{}:({})'",
            zsh_escape(long),
            zsh_escape(option.about),
            zsh_escape(value_name),
            words(option.values.iter().copied())
        ),
        (Some(long), _, Some(value_name)) => format!(
            "'--{}[{}]:{}:'",
            zsh_escape(long),
            zsh_escape(option.about),
            zsh_escape(value_name)
        ),
        (Some(long), _, None) => {
            format!("'--{}[{}]'", zsh_escape(long), zsh_escape(option.about))
        }
        (None, Some(short), None) => format!("'-{}[{}]'", short, zsh_escape(option.about)),
        _ => "'--help[Show help]'".to_string(),
    }
}

fn fish_option_line(option: &OptionSpec) -> String {
    let mut line = "complete -c cfd".to_string();

    if let Some(long) = option.long {
        line.push_str(&format!(" -l {}", fish_escape(long)));
    }

    if let Some(short) = option.short {
        line.push_str(&format!(" -s {short}"));
    }

    if option.value_name.is_some() {
        line.push_str(" -r -f");
    } else {
        line.push_str(" -f");
    }

    if !option.values.is_empty() {
        line.push_str(&format!(" -a '{}'", words(option.values.iter().copied())));
    }

    line.push_str(&format!(" -d '{}'", fish_escape(option.about)));
    line
}

fn fish_command_condition(parents: &[&str], siblings: &[&str]) -> String {
    if parents.is_empty() {
        "__fish_use_subcommand".to_string()
    } else {
        let mut checks = parents
            .iter()
            .map(|parent| format!("__fish_seen_subcommand_from {}", fish_escape(parent)))
            .collect::<Vec<_>>();

        checks.push(format!(
            "not __fish_seen_subcommand_from {}",
            siblings
                .iter()
                .map(|sibling| fish_escape(sibling))
                .collect::<Vec<_>>()
                .join(" ")
        ));

        checks.join("; and ")
    }
}

fn command_names(commands: &[CommandSpec]) -> Vec<&str> {
    commands.iter().map(|command| command.name).collect()
}

fn words<I>(values: I) -> String
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    values
        .into_iter()
        .map(|value| value.as_ref().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn zsh_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "'\\''")
}

fn fish_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(shell: &str) -> String {
        render_completion(shell, &crate::cli_spec::cli_spec()).unwrap()
    }

    #[test]
    fn bash_output_contains_expected_markers() {
        let output = render("bash");

        assert!(!output.is_empty());
        assert!(output.contains("_cfd"));
        assert!(output.contains("complete -F _cfd cfd"));
        assert!(output.contains("workspace"));
        assert!(output.contains("entry"));
        assert!(output.contains("--format"));
        assert!(output.contains("json"));
        assert!(output.contains("--workspace"));
    }

    #[test]
    fn zsh_output_contains_expected_markers() {
        let output = render("zsh");

        assert!(!output.is_empty());
        assert!(output.contains("#compdef cfd"));
        assert!(output.contains("_cfd"));
        assert!(output.contains("workspace"));
        assert!(output.contains("entry"));
        assert!(output.contains("--format"));
        assert!(output.contains("json"));
    }

    #[test]
    fn zsh_output_contains_context_specific_command_cases() {
        let output = render("zsh");

        assert!(output.contains("\"timer\""));
        assert!(output.contains("'current:Show running timer'"));
        assert!(output.contains("'start:Start timer'"));
        assert!(output.contains("'stop:Stop timer'"));
        assert!(output.contains("\"entry text\""));
        assert!(output.contains("'list:List known entry texts'"));
    }

    #[test]
    fn fish_output_contains_expected_markers() {
        let output = render("fish");

        assert!(!output.is_empty());
        assert!(output.contains("complete -c cfd"));
        assert!(output.contains("workspace"));
        assert!(output.contains("entry"));
        assert!(output.contains("--format"));
        assert!(output.contains("json"));
    }

    #[test]
    fn rounding_values_appear_in_generated_output() {
        for shell in crate::cli_spec::COMPLETION_SHELLS {
            let output = render(shell);

            for value in crate::cli_spec::ROUNDING_VALUES {
                assert!(
                    output.contains(value),
                    "{shell} completion output is missing rounding value {value}"
                );
            }
        }
    }

    #[test]
    fn unsupported_shell_returns_error() {
        let error = render_completion("powershell", &crate::cli_spec::cli_spec()).unwrap_err();

        assert!(error.to_string().contains("unsupported completion shell"));
        assert!(error.to_string().contains("powershell"));
    }
}
