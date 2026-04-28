mod support;

use std::process::Command;

use support::{bin, stderr, stdout};

fn completion_output(shell: &str) -> std::process::Output {
    let temp = tempfile::tempdir().unwrap();
    let missing_config = temp.path().join("missing").join("config.json");

    bin()
        .args(["completion", shell])
        .env("CFD_CONFIG", missing_config)
        .env_remove("CLOCKIFY_API_KEY")
        .output()
        .unwrap()
}

#[test]
fn bash_completion_succeeds() {
    let output = completion_output("bash");

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());

    let completion = stdout(&output);
    assert!(completion.contains("_cfd"));
    assert!(completion.contains("complete -F _cfd cfd"));
    assert!(completion.contains("workspace"));
    assert!(completion.contains("skill"));
    assert!(completion.contains("entry"));
    assert!(completion.contains("--format"));
    assert!(completion.contains("--workspace"));
    assert!(completion.contains("--project"));
    assert!(completion.contains("json"));
}

#[test]
fn generated_bash_completion_is_context_aware_when_sourced() {
    let output = completion_output("bash");
    assert!(output.status.success());

    let temp = tempfile::tempdir().unwrap();
    let completion_path = temp.path().join("cfd.bash");
    std::fs::write(&completion_path, stdout(&output)).unwrap();

    let script = format!(
        r#"
source {}

COMP_WORDS=(cfd "")
COMP_CWORD=1
_cfd
printf 'top:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd timer "")
COMP_CWORD=2
_cfd
printf 'timer:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd --workspace w1 "")
COMP_CWORD=3
_cfd
printf 'after-global-option:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd timer --workspace w1 "")
COMP_CWORD=4
_cfd
printf 'timer-after-option:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd entry text "")
COMP_CWORD=3
_cfd
printf 'entry-text:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd config set rounding "")
COMP_CWORD=4
_cfd
printf 'rounding:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd skill --scope "")
COMP_CWORD=3
_cfd
printf 'scope:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd today --sort "")
COMP_CWORD=3
_cfd
printf 'today-sort:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd entry list --sort "")
COMP_CWORD=4
_cfd
printf 'entry-sort:%s\n' "${{COMPREPLY[*]}}"

COMP_WORDS=(cfd status --week-start "")
COMP_CWORD=3
_cfd
printf 'status-week-start:%s\n' "${{COMPREPLY[*]}}"
"#,
        completion_path.display()
    );

    let complete_output = Command::new("bash")
        .args(["--noprofile", "--norc", "-c", &script])
        .output()
        .unwrap();

    assert!(
        complete_output.status.success(),
        "{}",
        String::from_utf8_lossy(&complete_output.stderr)
    );

    let candidates = String::from_utf8_lossy(&complete_output.stdout);
    assert!(candidates.contains("top:"));
    assert!(candidates.contains("workspace"));
    assert!(candidates.contains("timer:current start stop"));
    assert!(candidates.contains("after-global-option:help login logout skill whoami workspace config alias project client tag task entry today status timer completion"));
    assert!(candidates.contains("timer-after-option:current start stop"));
    assert!(candidates.contains("entry-text:list"));
    assert!(candidates.contains("rounding:off 1m 5m 10m 15m"));
    assert!(candidates.contains("scope:brief standard full"));
    assert!(candidates.contains("today-sort:asc desc"));
    assert!(candidates.contains("entry-sort:asc desc"));
    assert!(candidates.contains("status-week-start:monday sunday"));
    assert!(!candidates.contains("timer:add"));
}

#[test]
fn zsh_completion_succeeds() {
    let output = completion_output("zsh");

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());

    let completion = stdout(&output);
    assert!(completion.contains("#compdef cfd"));
    assert!(completion.contains("_cfd"));
    assert!(completion.contains("workspace"));
    assert!(completion.contains("skill"));
    assert!(completion.contains("entry"));
    assert!(completion.contains("--format"));
    assert!(completion.contains("--project"));
    assert!(completion.contains("json"));
}

#[test]
fn fish_completion_succeeds() {
    let output = completion_output("fish");

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());

    let completion = stdout(&output);
    assert!(completion.contains("complete -c cfd"));
    assert!(completion.contains("workspace"));
    assert!(completion.contains("skill"));
    assert!(completion.contains("entry"));
    assert!(completion.contains("--format"));
    assert!(completion.contains("-l project"));
    assert!(completion.contains("json"));
}

#[test]
fn generated_fish_completion_produces_candidates_when_sourced() {
    if Command::new("fish").arg("--version").output().is_err() {
        return;
    }

    let output = completion_output("fish");
    assert!(output.status.success());

    let temp = tempfile::tempdir().unwrap();
    let completion_path = temp.path().join("cfd.fish");
    std::fs::write(&completion_path, stdout(&output)).unwrap();

    let complete_output = Command::new("fish")
        .args([
            "--no-config",
            "-c",
            &format!(
                "source {}; complete -C 'cfd '; complete -C 'cfd entry text '; complete -C 'cfd config set '; complete -C 'cfd skill --scope '; complete -C 'cfd --format '; complete -C 'cfd today --sort '; complete -C 'cfd entry list --sort '; complete -C 'cfd status --week-start '",
                completion_path.display()
            ),
        ])
        .output()
        .unwrap();

    assert!(
        complete_output.status.success(),
        "{}",
        String::from_utf8_lossy(&complete_output.stderr)
    );

    let candidates = String::from_utf8_lossy(&complete_output.stdout);
    assert!(candidates.contains("workspace\tManage workspaces"));
    assert!(candidates.contains("skill\tPrint SKILL.md guidance"));
    assert!(candidates.contains("list\tList known entry texts"));
    assert!(candidates.contains("rounding\tRounding default"));
    assert!(candidates.contains("brief"));
    assert!(candidates.contains("standard"));
    assert!(candidates.contains("full"));
    assert!(candidates.contains("json"));
    assert!(candidates.contains("asc"));
    assert!(candidates.contains("desc"));
    assert!(candidates.contains("monday"));
    assert!(candidates.contains("sunday"));
    assert!(!candidates.contains("add\tCreate time entry"));
    assert!(!candidates.contains("interactive\tInteractively update stored defaults"));
}

#[test]
fn completion_output_has_exactly_one_trailing_newline() {
    for shell in ["bash", "zsh", "fish"] {
        let output = completion_output(shell);
        let completion = stdout(&output);

        assert!(
            completion.ends_with('\n'),
            "{shell} completion should end with a newline"
        );
        assert!(
            !completion.ends_with("\n\n"),
            "{shell} completion should not end with multiple newlines"
        );
    }
}

#[test]
fn invalid_completion_shell_exits_non_zero_with_useful_error() {
    let output = completion_output("powershell");

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("unsupported completion shell: powershell"));
}

#[test]
fn completion_rejects_extra_arguments() {
    let output = bin()
        .args(["completion", "bash", "extra"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("unknown command: cfd completion bash extra"));
}

#[test]
fn completion_help_documents_supported_shells_and_runtime_behavior() {
    for args in [["help", "completion"], ["completion", "help"]] {
        let output = bin().args(args).output().unwrap();

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let help = stdout(&output);
        assert!(help.contains("cfd completion <bash|zsh|fish>"));
        assert!(help.contains("Bash"));
        assert!(help.contains("Zsh"));
        assert!(help.contains("Fish"));
        assert!(help.contains("stdout"));
    }
}
