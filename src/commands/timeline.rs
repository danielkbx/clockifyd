use std::collections::{BTreeMap, BTreeSet};
use std::io::{stdout, IsTerminal, Stdout, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use crossterm::{
    cursor as term_cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::{entry, split, timer};
use crate::config;
use crate::error::CfdError;
use crate::types::{EntryFilters, OverlapWarning, RoundingMode, StoredConfig, TimeEntry};

const DEFAULT_DAY_START_HOUR: i64 = 8;
const DEFAULT_DAY_END_HOUR: i64 = 18;
const MIN_TERMINAL_WIDTH: usize = 40;
const LEGEND_HEIGHT: u16 = 7;
const ROWS_PER_DAY: usize = 2;
const BATCH_DAY_COUNT: usize = 7;
const LABEL_INSET: usize = 1;
const LABEL_PAD: usize = 1;
const SHORTCUT_BAR_COLOR: Color = Color::White;
const CURSOR_ENTRY_HIGHLIGHT_COLOR: Color = Color::Yellow;
const CURSOR_ROW_BACKGROUND: Color = Color::DarkGrey;
const CURSOR_ROW_FOREGROUND: Color = Color::White;

pub fn execute<T: HttpTransport + Clone + Send + 'static>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config: &StoredConfig,
) -> Result<(), CfdError> {
    reject_format_flags(args)?;
    if !stdout().is_terminal() {
        return Err(CfdError::message(
            "cfd timeline requires an interactive terminal (TTY)",
        ));
    }
    let rounding = config::resolve_rounding(args.no_rounding, config)?;
    let step = step_minutes_from_rounding(rounding);
    run_interactive(
        client,
        workspace_id,
        config,
        step,
        args.no_rounding,
        args.yes,
    )
}

fn reject_format_flags(args: &ParsedArgs) -> Result<(), CfdError> {
    if args.flags.contains_key("format") || args.flags.contains_key("columns") {
        return Err(CfdError::message(
            "cfd timeline is interactive and does not support --format or --columns",
        ));
    }
    Ok(())
}

fn step_minutes_from_rounding(mode: RoundingMode) -> i64 {
    match mode {
        RoundingMode::Off => 15,
        RoundingMode::OneMinute => 1,
        RoundingMode::FiveMinutes => 5,
        RoundingMode::TenMinutes => 10,
        RoundingMode::FifteenMinutes => 15,
    }
}

struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self, CfdError> {
        terminal::enable_raw_mode().map_err(io_err)?;
        let mut out = stdout();
        if let Err(error) = execute!(out, EnterAlternateScreen, term_cursor::Hide) {
            let _ = terminal::disable_raw_mode();
            return Err(io_err(error));
        }
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(stdout(), term_cursor::Show, LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();
            prev_hook(info);
        }));
        Ok(Self { active: true })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let _ = execute!(stdout(), term_cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        let _ = std::panic::take_hook();
    }
}

fn run_interactive<T: HttpTransport + Clone + Send + 'static>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    config: &StoredConfig,
    step: i64,
    no_rounding: bool,
    yes: bool,
) -> Result<(), CfdError> {
    let _guard = TerminalGuard::enter()?;
    let mut out = stdout();
    let (_, rows) = terminal::size().map_err(io_err)?;
    let visible_count = compute_visible_days(rows as usize, usize::MAX);
    let mut state = initial_state(client, workspace_id, visible_count)?;
    snap_cursor(&mut state, step);
    let loader = Loader::start(client.clone(), workspace_id.to_string());
    run_loop(
        &mut out,
        client,
        TimelineRuntime {
            workspace_id,
            config,
            loader: &loader,
            no_rounding,
            yes,
        },
        &mut state,
        step,
    )
}

struct TimelineRuntime<'a> {
    workspace_id: &'a str,
    config: &'a StoredConfig,
    loader: &'a Loader,
    no_rounding: bool,
    yes: bool,
}

fn run_loop<T: HttpTransport>(
    out: &mut Stdout,
    client: &ClockifyClient<T>,
    runtime: TimelineRuntime<'_>,
    state: &mut TimelineState,
    step: i64,
) -> Result<(), CfdError> {
    let mut dirty = true;
    let mut clear_next = true;
    let mut last_running_minute = state.now_minute();
    loop {
        if dirty {
            let (cols, rows) = terminal::size().map_err(io_err)?;
            let visible_count = compute_visible_days(rows as usize, usize::MAX);
            ensure_visible_days_requested(state, visible_count, runtime.loader);
            while let Ok(event) = runtime.loader.events.try_recv() {
                apply_loader_event(state, event);
            }
            adjust_viewport(state, visible_count);
            draw(
                out,
                state,
                cols as usize,
                rows as usize,
                visible_count,
                clear_next,
                step,
            )?;
            clear_next = false;
            dirty = false;
            last_running_minute = state.now_minute();
        }

        while let Ok(event) = runtime.loader.events.try_recv() {
            if apply_loader_event(state, event) {
                dirty = true;
            }
        }

        if event::poll(StdDuration::from_millis(100)).map_err(io_err)? {
            match event::read().map_err(io_err)? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    let (cols, _) = terminal::size().map_err(io_err)?;
                    match handle_key(key, state, step, cols as usize) {
                        Action::Quit => return Ok(()),
                        Action::Beep => {
                            queue!(out, Print('\x07')).map_err(io_err)?;
                            out.flush().map_err(io_err)?;
                        }
                        Action::Reload => {
                            reload_state(client, runtime.workspace_id, state, None)?;
                        }
                        Action::LoadOlder => {
                            let (_, rows) = terminal::size().map_err(io_err)?;
                            let visible_count =
                                compute_visible_days(rows as usize, state.days.len());
                            ensure_older_days_requested(state, visible_count, runtime.loader);
                        }
                        Action::StartTimerFromCursorEntry => execute_timeline_action(
                            client,
                            runtime.workspace_id,
                            runtime.config,
                            state,
                            TimelineAction::StartTimerFromCursorEntry,
                            runtime.no_rounding,
                            false,
                        )?,
                        Action::SplitCursorEntry => execute_timeline_action(
                            client,
                            runtime.workspace_id,
                            runtime.config,
                            state,
                            TimelineAction::SplitCursorEntry,
                            runtime.no_rounding,
                            false,
                        )?,
                        Action::DeleteCursorEntry => execute_timeline_action(
                            client,
                            runtime.workspace_id,
                            runtime.config,
                            state,
                            TimelineAction::DeleteCursorEntry,
                            runtime.no_rounding,
                            false,
                        )?,
                        Action::CommitEdit => {
                            if execute_edit_commit(
                                client,
                                runtime.workspace_id,
                                runtime.config,
                                state,
                            )? {
                                queue!(out, Print('\x07')).map_err(io_err)?;
                                out.flush().map_err(io_err)?;
                            }
                        }
                        Action::StopCurrentTimer => {
                            if runtime.yes {
                                execute_timeline_action(
                                    client,
                                    runtime.workspace_id,
                                    runtime.config,
                                    state,
                                    TimelineAction::StopCurrentTimer,
                                    runtime.no_rounding,
                                    true,
                                )?;
                            } else {
                                state.set_confirmation(
                                    "Stop current timer? [Y/n]",
                                    TimelineAction::StopCurrentTimer,
                                    "Stop cancelled.",
                                );
                            }
                        }
                        Action::ConfirmYes => {
                            if let Some(action) = state.interaction.pending_action() {
                                execute_timeline_action(
                                    client,
                                    runtime.workspace_id,
                                    runtime.config,
                                    state,
                                    action,
                                    runtime.no_rounding,
                                    true,
                                )?;
                            }
                        }
                        Action::ConfirmNo => {
                            state.cancel_confirmation();
                        }
                        Action::Redraw
                        | Action::MoveCursorLeft
                        | Action::MoveCursorRight
                        | Action::SelectPreviousDay
                        | Action::SelectNextDay
                        | Action::JumpStart
                        | Action::JumpEnd
                        | Action::JumpNow
                        | Action::BeginMoveEntry
                        | Action::BeginAdjustStart
                        | Action::BeginAdjustEnd
                        | Action::EditStepLeft
                        | Action::EditStepRight
                        | Action::CancelEdit => {}
                    }
                    dirty = true;
                }
                Event::Resize(_, _) => {
                    clear_next = true;
                    dirty = true;
                }
                _ => {}
            }
        } else {
            if state.has_running_timer() {
                state.refresh_now();
                let current_minute = state.now_minute();
                if current_minute != last_running_minute {
                    recompute_bounds(state);
                    dirty = true;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Redraw,
    Beep,
    Quit,
    Reload,
    LoadOlder,
    MoveCursorLeft,
    MoveCursorRight,
    SelectPreviousDay,
    SelectNextDay,
    JumpStart,
    JumpEnd,
    JumpNow,
    StartTimerFromCursorEntry,
    SplitCursorEntry,
    DeleteCursorEntry,
    BeginMoveEntry,
    BeginAdjustStart,
    BeginAdjustEnd,
    EditStepLeft,
    EditStepRight,
    CommitEdit,
    CancelEdit,
    StopCurrentTimer,
    ConfirmYes,
    ConfirmNo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShortcutSegment {
    key: &'static str,
    label: &'static str,
    action: Action,
}

#[derive(Debug, Clone)]
struct ShortcutSet {
    top: Vec<ShortcutSegment>,
    bottom: Vec<ShortcutSegment>,
    accepted: Vec<ShortcutSegment>,
}

fn handle_key(key: KeyEvent, state: &mut TimelineState, step: i64, cols: usize) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return Action::Quit;
    }

    let shortcuts = build_shortcuts(state, cols, step);
    let Some(action) = action_for_key(key, &shortcuts.accepted) else {
        return Action::Beep;
    };

    match action {
        Action::MoveCursorLeft => {
            state.move_cursor(-step);
            Action::Redraw
        }
        Action::MoveCursorRight => {
            state.move_cursor(step);
            Action::Redraw
        }
        Action::BeginMoveEntry => {
            if state.start_edit(EditMode::Move) {
                Action::Redraw
            } else {
                Action::Beep
            }
        }
        Action::BeginAdjustStart => {
            if state.start_edit(EditMode::Start) {
                Action::Redraw
            } else {
                Action::Beep
            }
        }
        Action::BeginAdjustEnd => {
            if state.start_edit(EditMode::End) {
                Action::Redraw
            } else {
                Action::Beep
            }
        }
        Action::EditStepLeft => {
            if state.apply_edit_step(-step) {
                Action::Redraw
            } else {
                Action::Beep
            }
        }
        Action::EditStepRight => {
            if state.apply_edit_step(step) {
                Action::Redraw
            } else {
                Action::Beep
            }
        }
        Action::CancelEdit => {
            state.cancel_edit();
            Action::Redraw
        }
        Action::SelectPreviousDay => {
            if state.selected_day > 0 {
                state.selected_day -= 1;
                Action::Redraw
            } else {
                Action::LoadOlder
            }
        }
        Action::SelectNextDay => {
            if state.selected_day + 1 < state.days.len() {
                state.selected_day += 1;
            }
            Action::Redraw
        }
        Action::JumpStart => {
            state.cursor_minute = state.start_minute;
            Action::Redraw
        }
        Action::JumpEnd => {
            state.cursor_minute = state.end_minute;
            Action::Redraw
        }
        Action::JumpNow => {
            state.refresh_now();
            state.selected_day = state.days.len() - 1;
            let now_minute = state.now_minute();
            state.cursor_minute = now_minute.clamp(state.start_minute, state.end_minute);
            snap_cursor(state, step);
            Action::Redraw
        }
        other => other,
    }
}

fn action_for_key(key: KeyEvent, segments: &[ShortcutSegment]) -> Option<Action> {
    segments
        .iter()
        .find(|segment| shortcut_matches_key(segment.key, key))
        .map(|segment| action_for_shortcut_key(segment, key))
}

fn action_for_shortcut_key(segment: &ShortcutSegment, key: KeyEvent) -> Action {
    match segment.key {
        "←/→" if matches!(key.code, KeyCode::Right) && segment.action == Action::EditStepLeft => {
            Action::EditStepRight
        }
        "←/→" if matches!(key.code, KeyCode::Right) => Action::MoveCursorRight,
        "↑/↓" if matches!(key.code, KeyCode::Down) => Action::SelectNextDay,
        "Home/End" if matches!(key.code, KeyCode::End) => Action::JumpEnd,
        _ => segment.action,
    }
}

fn shortcut_matches_key(shortcut: &str, key: KeyEvent) -> bool {
    match shortcut {
        "←/→" => matches!(key.code, KeyCode::Left | KeyCode::Right),
        "↑/↓" => matches!(key.code, KeyCode::Up | KeyCode::Down),
        "Home/End" => matches!(key.code, KeyCode::Home | KeyCode::End),
        "Ctrl-C" => {
            key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c'))
        }
        "q/Esc/Ctrl-C" => {
            matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                || key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c'))
        }
        "y/Enter" => matches!(
            key.code,
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter
        ),
        "Enter" => matches!(key.code, KeyCode::Enter),
        "Esc" => matches!(key.code, KeyCode::Esc),
        "n/Esc" => matches!(
            key.code,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc
        ),
        value if value.len() == 1 => {
            let expected = value.chars().next().unwrap();
            matches!(key.code, KeyCode::Char(actual) if actual.eq_ignore_ascii_case(&expected))
        }
        _ => false,
    }
}

fn execute_timeline_action<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    config: &StoredConfig,
    state: &mut TimelineState,
    action: TimelineAction,
    no_rounding: bool,
    allow_overlap: bool,
) -> Result<(), CfdError> {
    match action {
        TimelineAction::StartTimerFromCursorEntry => {
            if state.has_running_timer() {
                state.set_message("Timer already running; stop it before starting another.");
                return Ok(());
            }
            let Some(entry) = selected_loaded_entry(state).cloned() else {
                return Ok(());
            };
            let Some(project_id) = entry.project_id.clone() else {
                state.set_message("Selected entry has no project; cannot start timer.");
                return Ok(());
            };
            execute_timeline_action(
                client,
                workspace_id,
                config,
                state,
                TimelineAction::StartTimerFromEntry {
                    entry_id: entry.id.clone(),
                },
                no_rounding,
                allow_overlap,
            )?;
            if matches!(state.interaction, InteractionState::Idle { .. }) {
                let _ = project_id;
            }
            Ok(())
        }
        TimelineAction::SplitCursorEntry => {
            let Some(entry) = selected_loaded_entry(state).cloned() else {
                return Ok(());
            };
            if entry.running {
                state.set_message("Cannot split the running timer entry here.");
                return Ok(());
            }
            let rounding = config::resolve_rounding(no_rounding, config)?;
            let step = step_minutes_from_rounding(rounding);
            if !entry_can_split_at_cursor(&entry, state.cursor_minute, step) {
                state.set_message("Entry cannot be split at the current cursor position.");
                return Ok(());
            }
            let at = cursor_timestamp(state)?;
            execute_timeline_action(
                client,
                workspace_id,
                config,
                state,
                TimelineAction::SplitEntryAtCursor {
                    entry_id: entry.id,
                    at,
                },
                no_rounding,
                allow_overlap,
            )
        }
        TimelineAction::DeleteCursorEntry => {
            let Some(entry) = selected_loaded_entry(state).cloned() else {
                return Ok(());
            };
            state.set_confirmation(
                format!("Delete entry {}? [Y/n]", entry.id),
                TimelineAction::DeleteEntry { entry_id: entry.id },
                "Delete cancelled.",
            );
            Ok(())
        }
        TimelineAction::StopCurrentTimer => {
            let mut overlap_ids = Vec::new();
            let result = timer::stop_timer_entry(
                client,
                workspace_id,
                config,
                None,
                no_rounding,
                |warning| confirm_or_capture_overlap(warning, allow_overlap, &mut overlap_ids),
            );
            match result {
                Ok(stopped) => {
                    reload_state(
                        client,
                        workspace_id,
                        state,
                        Some(format!("Stopped timer {}.", stopped.id)),
                    )?;
                }
                Err(error) if !allow_overlap && !overlap_ids.is_empty() => {
                    state.set_confirmation(
                        format!(
                            "Overlaps existing entries: {}. Stop timer anyway? [Y/n]",
                            overlap_ids.join(", ")
                        ),
                        TimelineAction::StopCurrentTimer,
                        "Stop cancelled.",
                    );
                    let _ = error;
                }
                Err(error) => state.set_message(format_timeline_error(error)),
            }
            Ok(())
        }
        TimelineAction::StartTimerFromEntry { entry_id } => {
            let Some(entry) = find_entry_view(state, &entry_id).cloned() else {
                state.set_message("Selected entry is no longer visible.");
                return Ok(());
            };
            let Some(project_id) = entry.project_id.clone() else {
                state.set_message("Selected entry has no project; cannot start timer.");
                return Ok(());
            };
            let fields = timer::TimerStartFields {
                project_id,
                task_id: entry.task_id.clone(),
                tag_ids: entry.tag_ids.clone(),
                description: (!entry.description.is_empty()).then_some(entry.description.clone()),
            };
            let mut overlap_ids = Vec::new();
            let result = timer::start_timer_entry_with_fields(
                client,
                workspace_id,
                config,
                fields,
                None,
                no_rounding,
                |warning| confirm_or_capture_overlap(warning, allow_overlap, &mut overlap_ids),
            );
            match result {
                Ok(created) => {
                    reload_state(
                        client,
                        workspace_id,
                        state,
                        Some(format!("Started timer {}.", created.id)),
                    )?;
                }
                Err(error) if !allow_overlap && !overlap_ids.is_empty() => {
                    state.set_confirmation(
                        format!(
                            "Overlaps existing entries: {}. Start timer anyway? [Y/n]",
                            overlap_ids.join(", ")
                        ),
                        TimelineAction::StartTimerFromEntry { entry_id },
                        "Start cancelled.",
                    );
                    let _ = error;
                }
                Err(error) => state.set_message(format_timeline_error(error)),
            }
            Ok(())
        }
        TimelineAction::SplitEntryAtCursor { entry_id, at } => {
            let mut overlap_ids = Vec::new();
            let result = split::split_entry_at(
                client,
                workspace_id,
                config,
                split::SplitEntryAt {
                    entry_id: &entry_id,
                    at_input: &at,
                    gap_input: None,
                    no_rounding,
                },
                |warning| confirm_or_capture_overlap(warning, allow_overlap, &mut overlap_ids),
            );
            match result {
                Ok(result) => {
                    reload_state(
                        client,
                        workspace_id,
                        state,
                        Some(format!(
                            "Split entry {}; created {}.",
                            result.updated.id, result.created.id
                        )),
                    )?;
                }
                Err(error) if !allow_overlap && !overlap_ids.is_empty() => {
                    state.set_confirmation(
                        format!(
                            "Overlaps existing entries: {}. Split anyway? [Y/n]",
                            overlap_ids.join(", ")
                        ),
                        TimelineAction::SplitEntryAtCursor { entry_id, at },
                        "Split cancelled.",
                    );
                    let _ = error;
                }
                Err(error) => state.set_message(format_timeline_error(error)),
            }
            Ok(())
        }
        TimelineAction::DeleteEntry { entry_id } => {
            match client.delete_time_entry(workspace_id, &entry_id) {
                Ok(()) => {
                    reload_state(
                        client,
                        workspace_id,
                        state,
                        Some(format!("Deleted entry {entry_id}.")),
                    )?;
                    Ok(())
                }
                Err(error) => {
                    state.set_message(format_timeline_error(error));
                    Ok(())
                }
            }
        }
    }
}

fn confirm_or_capture_overlap(
    warning: &OverlapWarning,
    allow_overlap: bool,
    overlap_ids: &mut Vec<String>,
) -> Result<bool, CfdError> {
    if allow_overlap {
        Ok(true)
    } else {
        *overlap_ids = warning.overlapping_ids.clone();
        Ok(false)
    }
}

fn execute_edit_commit<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    config_state: &StoredConfig,
    state: &mut TimelineState,
) -> Result<bool, CfdError> {
    let Some(session) = state.edit.clone() else {
        return Ok(false);
    };
    if edit_session_is_noop(&session) {
        state.edit = None;
        state.set_message("No changes.");
        return Ok(false);
    }

    let start = minute_timestamp(session.date, session.draft_start_minute);
    let end_minute = edit_effective_end_minute(state, &session);
    let end = minute_timestamp(session.date, end_minute);
    let mut overlap_ids = Vec::new();
    let result = if session.running {
        entry::update_entry_start_exact(
            client,
            workspace_id,
            entry::ExactEntryStartUpdate {
                entry_id: &session.entry_id,
                start: &start,
                overlap_end: Some(&end),
            },
            |warning| {
                overlap_ids = warning.overlapping_ids.clone();
                Ok(false)
            },
        )
    } else {
        entry::update_entry_times_exact(
            client,
            workspace_id,
            entry::ExactEntryTimeUpdate {
                entry_id: &session.entry_id,
                start: &start,
                end: &end,
            },
            |warning| {
                overlap_ids = warning.overlapping_ids.clone();
                Ok(false)
            },
        )
    };

    match result {
        Ok(updated) => {
            update_active_switch_start_if_needed(config_state, workspace_id, &updated.id, &start)?;
            state.edit = None;
            reload_day_state(
                client,
                workspace_id,
                state,
                session.date,
                Some(format!("Updated entry {}.", updated.id)),
            )?;
        }
        Err(error) if !overlap_ids.is_empty() => {
            state.set_message(format!(
                "Edit would overlap existing entries: {}.",
                overlap_ids.join(", ")
            ));
            let _ = error;
            return Ok(true);
        }
        Err(error) => {
            state.set_message(format_timeline_error(error));
        }
    }
    Ok(false)
}

fn edit_session_is_noop(session: &EditSession) -> bool {
    session.draft_start_minute == session.original_start_minute
        && (session.running || session.draft_end_minute == session.original_end_minute)
}

fn update_active_switch_start_if_needed(
    config_state: &StoredConfig,
    workspace_id: &str,
    entry_id: &str,
    start: &str,
) -> Result<(), CfdError> {
    let Some(active_switch) = config_state.active_switch.as_ref() else {
        return Ok(());
    };
    if active_switch.workspace_id != workspace_id || active_switch.switched_entry_id != entry_id {
        return Ok(());
    }
    if let Some(next_config) = config_with_updated_active_switch_start(config_state, start) {
        config::save_config(&next_config)?;
    }
    Ok(())
}

fn config_with_updated_active_switch_start(
    config_state: &StoredConfig,
    start: &str,
) -> Option<StoredConfig> {
    let mut next_config = config_state.clone();
    let active_switch = next_config.active_switch.as_mut()?;
    active_switch.switched_start = start.to_owned();
    Some(next_config)
}

fn selected_loaded_entry(state: &mut TimelineState) -> Option<&EntryView> {
    if state.selected().load_status != DayLoadStatus::Loaded {
        state.set_message("No loaded entry at cursor.");
        return None;
    }
    if entry_at_cursor(state).is_none() {
        state.set_message("No entry at cursor.");
        return None;
    }
    entry_at_cursor(state)
}

fn find_entry_view<'a>(state: &'a TimelineState, entry_id: &str) -> Option<&'a EntryView> {
    state
        .days
        .iter()
        .flat_map(|day| day.entries.iter())
        .find(|entry| entry.id == entry_id)
}

fn cursor_timestamp(state: &TimelineState) -> Result<String, CfdError> {
    Ok(minute_timestamp(state.selected().date, state.cursor_minute))
}

fn minute_timestamp(date: NaiveDate, minute: i64) -> String {
    let day_start = local_at(date, 0, 0);
    (day_start + Duration::minutes(minute)).to_rfc3339()
}

fn reload_state<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    state: &mut TimelineState,
    message: Option<String>,
) -> Result<(), CfdError> {
    let preserved_cursor = state.cursor_minute;
    let preserved_date = state.selected().date;
    let next_generation = state.generation + 1;
    let (_, rows) = terminal::size().map_err(io_err)?;
    let visible_count = compute_visible_days(rows as usize, usize::MAX);
    *state = initial_state(client, workspace_id, visible_count)?;
    state.generation = next_generation;
    state.cursor_minute = preserved_cursor.clamp(state.start_minute, state.end_minute);
    preserve_selected_date(state, preserved_date);
    if let Some(message) = message {
        state.set_message(message);
    }
    Ok(())
}

fn reload_day_state<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    state: &mut TimelineState,
    date: NaiveDate,
    message: Option<String>,
) -> Result<(), CfdError> {
    let preserved_cursor = state.cursor_minute;
    let preserved_date = state.selected().date;
    let loaded_days = fetch_range(client, workspace_id, date, 1, Utc::now())?;
    merge_days(&mut state.days, loaded_days);
    preserve_selected_date(state, preserved_date);
    recompute_bounds(state);
    state.cursor_minute = preserved_cursor.clamp(state.start_minute, state.end_minute);
    if let Some(message) = message {
        state.set_message(message);
    }
    Ok(())
}

fn format_timeline_error(error: CfdError) -> String {
    let message = error.to_string();
    if message == "timer already running" {
        "Timer already running.".into()
    } else if message == "no running timer" {
        "No running timer.".into()
    } else {
        message
    }
}

fn io_err(error: std::io::Error) -> CfdError {
    CfdError::message(format!("terminal IO error: {error}"))
}

fn compute_visible_days(rows: usize, total_days: usize) -> usize {
    let chrome_rows = 1 + 1 + 1 + 1 + 1 + LEGEND_HEIGHT as usize + 1;
    let avail = rows.saturating_sub(chrome_rows);
    let day_rows = avail / ROWS_PER_DAY;
    day_rows.max(1).min(total_days.max(1))
}

fn adjust_viewport(state: &mut TimelineState, visible_count: usize) {
    if visible_count == 0 {
        return;
    }
    if state.selected_day < state.viewport_top {
        state.viewport_top = state.selected_day;
    }
    if state.selected_day >= state.viewport_top + visible_count {
        state.viewport_top = state.selected_day + 1 - visible_count;
    }
    let max_top = state.days.len().saturating_sub(visible_count);
    if state.viewport_top > max_top {
        state.viewport_top = max_top;
    }
}

fn snap_cursor(state: &mut TimelineState, step: i64) {
    if step <= 0 {
        return;
    }
    let offset = state.cursor_minute - state.start_minute;
    let snapped = state.start_minute + ((offset + step / 2) / step) * step;
    state.cursor_minute = snapped.clamp(state.start_minute, state.end_minute);
}

#[derive(Debug, Clone)]
struct TimelineState {
    days: Vec<DayView>,
    selected_day: usize,
    viewport_top: usize,
    start_minute: i64,
    end_minute: i64,
    cursor_minute: i64,
    now: DateTime<Utc>,
    current_timer_id: Option<String>,
    loading: LoadingState,
    interaction: InteractionState,
    edit: Option<EditSession>,
    generation: u64,
}

impl TimelineState {
    fn now_minute(&self) -> i64 {
        let local = self.now.with_timezone(&Local);
        if local.date_naive() == self.today_local() {
            i64::from(local.hour()) * 60 + i64::from(local.minute())
        } else {
            self.end_minute
        }
    }

    fn today_local(&self) -> NaiveDate {
        self.days
            .last()
            .map(|day| day.date)
            .unwrap_or_else(|| Local::now().date_naive())
    }

    fn move_cursor(&mut self, delta: i64) {
        let target = self.cursor_minute + delta;
        self.cursor_minute = target.clamp(self.start_minute, self.end_minute);
    }

    fn refresh_now(&mut self) {
        self.now = Utc::now();
    }

    fn selected(&self) -> &DayView {
        debug_assert!(
            !self.days.is_empty(),
            "TimelineState.days must never be empty"
        );
        let idx = self.selected_day.min(self.days.len().saturating_sub(1));
        &self.days[idx]
    }

    fn has_running_timer(&self) -> bool {
        self.current_timer_id.is_some()
    }

    fn set_message(&mut self, message: impl Into<String>) {
        self.interaction = InteractionState::Idle {
            message: Some(message.into()),
        };
    }

    fn set_confirmation(
        &mut self,
        prompt: impl Into<String>,
        action: TimelineAction,
        on_no_message: impl Into<String>,
    ) {
        self.interaction = InteractionState::Confirm {
            prompt: prompt.into(),
            action,
            on_no_message: on_no_message.into(),
        };
    }

    fn cancel_confirmation(&mut self) {
        let message = match &self.interaction {
            InteractionState::Confirm { on_no_message, .. } => Some(on_no_message.clone()),
            InteractionState::Idle { message } => message.clone(),
        };
        self.interaction = InteractionState::Idle { message };
    }

    fn start_edit(&mut self, mode: EditMode) -> bool {
        if self.edit.is_some() || self.selected().load_status != DayLoadStatus::Loaded {
            return false;
        }
        let Some(entry) = entry_at_cursor(self).cloned() else {
            return false;
        };
        if entry.running && mode != EditMode::Start {
            return false;
        }
        let end_minute = if entry.running {
            self.now_minute().max(entry.start_minute)
        } else {
            entry.end_minute
        };
        let session = EditSession {
            entry_id: entry.id,
            date: self.selected().date,
            mode,
            running: entry.running,
            original_start_minute: entry.start_minute,
            original_end_minute: end_minute,
            draft_start_minute: entry.start_minute,
            draft_end_minute: end_minute,
            original_cursor_minute: self.cursor_minute,
        };
        self.cursor_minute = match mode {
            EditMode::Move => self.cursor_minute,
            EditMode::Start => session.draft_start_minute,
            EditMode::End => session.draft_end_minute,
        };
        self.edit = Some(session);
        self.interaction = InteractionState::Idle { message: None };
        true
    }

    fn apply_edit_step(&mut self, delta: i64) -> bool {
        let step = if delta < 0 { -delta } else { delta }.max(1);
        let Some(session) = self.edit.clone() else {
            return false;
        };
        let delta = if delta < 0 { -step } else { step };
        let (draft_start, draft_end) = edit_candidate(
            session.mode,
            session.draft_start_minute,
            edit_effective_end_minute(self, &session),
            delta,
        );
        if !edit_interval_is_valid(self, &session, draft_start, draft_end) {
            return false;
        }
        let Some(edit) = &mut self.edit else {
            return false;
        };
        edit.draft_start_minute = draft_start;
        edit.draft_end_minute = draft_end;
        self.cursor_minute = match edit.mode {
            EditMode::Move => {
                (self.cursor_minute + delta).clamp(self.start_minute, self.end_minute)
            }
            EditMode::Start => draft_start,
            EditMode::End => draft_end,
        };
        self.interaction = InteractionState::Idle { message: None };
        true
    }

    fn cancel_edit(&mut self) {
        if let Some(session) = self.edit.take() {
            self.cursor_minute = session.original_cursor_minute;
        }
        self.interaction = InteractionState::Idle { message: None };
    }
}

#[derive(Debug, Clone)]
enum InteractionState {
    Idle {
        message: Option<String>,
    },
    Confirm {
        prompt: String,
        action: TimelineAction,
        on_no_message: String,
    },
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::Idle { message: None }
    }
}

impl InteractionState {
    fn is_confirming(&self) -> bool {
        matches!(self, Self::Confirm { .. })
    }

    fn pending_action(&self) -> Option<TimelineAction> {
        match self {
            Self::Confirm { action, .. } => Some(action.clone()),
            Self::Idle { .. } => None,
        }
    }

    fn line(&self) -> Option<&str> {
        match self {
            Self::Idle { message } => message.as_deref(),
            Self::Confirm { prompt, .. } => Some(prompt.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditMode {
    Move,
    Start,
    End,
}

#[derive(Debug, Clone)]
struct EditSession {
    entry_id: String,
    date: NaiveDate,
    mode: EditMode,
    running: bool,
    original_start_minute: i64,
    original_end_minute: i64,
    draft_start_minute: i64,
    draft_end_minute: i64,
    original_cursor_minute: i64,
}

fn edit_candidate(mode: EditMode, start: i64, end: i64, delta: i64) -> (i64, i64) {
    match mode {
        EditMode::Move => (start + delta, end + delta),
        EditMode::Start => (start + delta, end),
        EditMode::End => (start, end + delta),
    }
}

fn edit_interval_is_valid(
    state: &TimelineState,
    session: &EditSession,
    start: i64,
    end: i64,
) -> bool {
    if start < 0 || end > 24 * 60 || end <= start {
        return false;
    }
    if session.running && (session.mode != EditMode::Start || end != state.now_minute()) {
        return false;
    }
    let Some(day) = state.days.iter().find(|day| day.date == session.date) else {
        return false;
    };
    if day.load_status != DayLoadStatus::Loaded {
        return false;
    }
    !day.entries
        .iter()
        .filter(|entry| entry.id != session.entry_id)
        .any(|entry| {
            let (entry_start, entry_end) = entry_display_interval_for_day(state, day.date, entry);
            entry_start < end && start < entry_end
        })
}

fn edit_action_available(
    state: &TimelineState,
    entry: &EntryView,
    mode: EditMode,
    step: i64,
) -> bool {
    if entry.running && mode != EditMode::Start {
        return false;
    }
    let step = step.max(1);
    let session = EditSession {
        entry_id: entry.id.clone(),
        date: state.selected().date,
        mode,
        running: entry.running,
        original_start_minute: entry.start_minute,
        original_end_minute: if entry.running {
            state.now_minute().max(entry.start_minute)
        } else {
            entry.end_minute
        },
        draft_start_minute: entry.start_minute,
        draft_end_minute: if entry.running {
            state.now_minute().max(entry.start_minute)
        } else {
            entry.end_minute
        },
        original_cursor_minute: state.cursor_minute,
    };
    [-step, step].into_iter().any(|delta| {
        let (start, end) = edit_candidate(
            mode,
            entry.start_minute,
            edit_effective_end_minute(state, &session),
            delta,
        );
        edit_interval_is_valid(state, &session, start, end)
    })
}

fn edit_effective_end_minute(state: &TimelineState, session: &EditSession) -> i64 {
    if session.running {
        state.now_minute().max(session.draft_start_minute)
    } else {
        session.draft_end_minute
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TimelineAction {
    StartTimerFromCursorEntry,
    SplitCursorEntry,
    DeleteCursorEntry,
    StartTimerFromEntry { entry_id: String },
    SplitEntryAtCursor { entry_id: String, at: String },
    DeleteEntry { entry_id: String },
    StopCurrentTimer,
}

#[derive(Debug, Clone)]
struct DayView {
    date: NaiveDate,
    entries: Vec<EntryView>,
    total_minutes: i64,
    load_status: DayLoadStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DayLoadStatus {
    Loading,
    Loaded,
    Failed,
}

#[derive(Debug, Clone)]
struct LoadingState {
    oldest_requested: NaiveDate,
    older_batch_in_flight: bool,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct EntryView {
    id: String,
    description: String,
    project_id: Option<String>,
    project_name: Option<String>,
    task_id: Option<String>,
    tag_ids: Vec<String>,
    start_minute: i64,
    end_minute: i64,
    running: bool,
}

struct Loader {
    requests: Sender<LoaderRequest>,
    events: Receiver<LoaderEvent>,
}

impl Loader {
    fn start<T>(client: ClockifyClient<T>, workspace_id: String) -> Self
    where
        T: HttpTransport + Send + 'static,
    {
        let (request_tx, request_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                match request {
                    LoaderRequest::LoadRange {
                        generation,
                        start,
                        day_count,
                    } => {
                        let result =
                            fetch_range(&client, &workspace_id, start, day_count, Utc::now());
                        let event = match result {
                            Ok(days) => LoaderEvent::RangeLoaded {
                                generation,
                                start,
                                days,
                            },
                            Err(error) => LoaderEvent::RangeFailed {
                                generation,
                                start,
                                day_count,
                                message: error.to_string(),
                            },
                        };
                        if event_tx.send(event).is_err() {
                            break;
                        }
                    }
                    LoaderRequest::Shutdown => break,
                }
            }
        });
        Self {
            requests: request_tx,
            events: event_rx,
        }
    }
}

impl Drop for Loader {
    fn drop(&mut self) {
        let _ = self.requests.send(LoaderRequest::Shutdown);
    }
}

enum LoaderRequest {
    LoadRange {
        generation: u64,
        start: NaiveDate,
        day_count: usize,
    },
    Shutdown,
}

enum LoaderEvent {
    RangeLoaded {
        generation: u64,
        start: NaiveDate,
        days: Vec<DayView>,
    },
    RangeFailed {
        generation: u64,
        start: NaiveDate,
        day_count: usize,
        message: String,
    },
}

fn initial_state<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    visible_count: usize,
) -> Result<TimelineState, CfdError> {
    let now = Utc::now();
    let today_local = now.with_timezone(&Local).date_naive();
    let visible_count = visible_count.max(1);
    let oldest_visible = today_local - Duration::days((visible_count - 1) as i64);
    let mut days = loading_days(oldest_visible, visible_count);
    let newest_batch_count = visible_count.min(BATCH_DAY_COUNT);
    let newest_batch_start = today_local - Duration::days((newest_batch_count - 1) as i64);
    let loaded_days = fetch_range(
        client,
        workspace_id,
        newest_batch_start,
        newest_batch_count,
        now,
    )?;
    let current_timer_id = load_current_timer_id(client, workspace_id)?;
    merge_days(&mut days, loaded_days);

    let (start_minute, end_minute) = compute_global_bounds(&days, now, today_local);
    let now_local = now.with_timezone(&Local);
    let now_minute = if now_local.date_naive() == today_local {
        (i64::from(now_local.hour()) * 60 + i64::from(now_local.minute()))
            .clamp(start_minute, end_minute)
    } else {
        end_minute
    };

    let selected_day = days.len() - 1;
    Ok(TimelineState {
        days,
        selected_day,
        viewport_top: 0,
        start_minute,
        end_minute,
        cursor_minute: now_minute,
        now,
        current_timer_id,
        loading: LoadingState {
            oldest_requested: newest_batch_start,
            older_batch_in_flight: false,
            last_error: None,
        },
        interaction: InteractionState::default(),
        edit: None,
        generation: 0,
    })
}

fn load_current_timer_id<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
) -> Result<Option<String>, CfdError> {
    let user = client.get_current_user()?;
    Ok(client
        .get_current_timers(workspace_id)?
        .into_iter()
        .find(|entry| entry.user_id.as_deref() == Some(user.id.as_str()))
        .map(|entry| entry.id))
}

fn load_project_names<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    entries: &[TimeEntry],
) -> Result<BTreeMap<String, String>, CfdError> {
    let project_ids: BTreeSet<&str> = entries
        .iter()
        .filter_map(|entry| entry.project_id.as_deref())
        .collect();
    if project_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let projects = client.list_projects(workspace_id)?;
    Ok(projects
        .into_iter()
        .filter(|project| project_ids.contains(project.id.as_str()))
        .map(|project| (project.id, project.name))
        .collect())
}

fn fetch_range<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    start_date: NaiveDate,
    day_count: usize,
    now: DateTime<Utc>,
) -> Result<Vec<DayView>, CfdError> {
    let day_count = day_count.max(1);
    let range_start_local = local_at(start_date, 0, 0);
    let range_end_local = range_start_local + Duration::days(day_count as i64);
    let range_start_utc = range_start_local.with_timezone(&Utc);
    let range_end_utc = range_end_local.with_timezone(&Utc);

    let filters = EntryFilters {
        start: Some(range_start_utc.to_rfc3339()),
        end: Some(range_end_utc.to_rfc3339()),
        ..EntryFilters::default()
    };
    let user = client.get_current_user()?;
    let entries = client.list_time_entries(workspace_id, &user.id, &filters)?;
    let project_names = load_project_names(client, workspace_id, &entries)?;

    let mut days = loaded_empty_days(start_date, day_count);
    for entry in &entries {
        let start = parse_rfc3339(&entry.time_interval.start)?;
        let bucket_date = start.with_timezone(&Local).date_naive();
        let bucket_offset = (bucket_date - start_date).num_days();
        if !(0..day_count as i64).contains(&bucket_offset) {
            continue;
        }
        let bucket_idx = bucket_offset as usize;
        let day = &days[bucket_idx];
        if let Some(view) = entry_view_for_day(entry, day.date, now, &project_names)? {
            days[bucket_idx].entries.push(view);
        }
    }
    finalize_days(&mut days);
    Ok(days)
}

fn entry_view_for_day(
    entry: &TimeEntry,
    date: NaiveDate,
    now: DateTime<Utc>,
    project_names: &BTreeMap<String, String>,
) -> Result<Option<EntryView>, CfdError> {
    let start = parse_rfc3339(&entry.time_interval.start)?;
    let (end, running) = match entry.time_interval.end.as_deref() {
        Some(value) => (parse_rfc3339(value)?, false),
        None => (now, true),
    };
    if start.with_timezone(&Local).date_naive() != date {
        return Ok(None);
    }
    let day_start_utc = local_at(date, 0, 0).with_timezone(&Utc);
    let day_end_utc = day_start_utc + Duration::days(1);
    let start_clamped = start.max(day_start_utc);
    let end_clamped = end.min(day_end_utc).max(start_clamped);
    let start_minute = (start_clamped - day_start_utc)
        .num_minutes()
        .clamp(0, 24 * 60);
    let end_minute = (end_clamped - day_start_utc)
        .num_minutes()
        .clamp(0, 24 * 60);
    Ok(Some(EntryView {
        id: entry.id.clone(),
        description: entry.description.clone(),
        project_id: entry.project_id.clone(),
        project_name: entry
            .project_id
            .as_deref()
            .and_then(|id| project_names.get(id).cloned()),
        task_id: entry.task_id.clone(),
        tag_ids: entry.tag_ids.clone(),
        start_minute,
        end_minute,
        running,
    }))
}

fn loading_days(start: NaiveDate, count: usize) -> Vec<DayView> {
    day_range(start, count, DayLoadStatus::Loading)
}

fn loaded_empty_days(start: NaiveDate, count: usize) -> Vec<DayView> {
    day_range(start, count, DayLoadStatus::Loaded)
}

fn day_range(start: NaiveDate, count: usize, load_status: DayLoadStatus) -> Vec<DayView> {
    (0..count)
        .map(|offset| DayView {
            date: start + Duration::days(offset as i64),
            entries: Vec::new(),
            total_minutes: 0,
            load_status,
        })
        .collect()
}

fn finalize_days(days: &mut [DayView]) {
    for day in days {
        day.entries.sort_by_key(|view| view.start_minute);
        day.total_minutes = day
            .entries
            .iter()
            .map(|entry| (entry.end_minute - entry.start_minute).max(0))
            .sum();
        day.load_status = DayLoadStatus::Loaded;
    }
}

fn merge_days(days: &mut Vec<DayView>, loaded: Vec<DayView>) {
    for day in loaded {
        match days.binary_search_by_key(&day.date, |existing| existing.date) {
            Ok(idx) => days[idx] = day,
            Err(idx) => days.insert(idx, day),
        }
    }
}

fn preserve_selected_date(state: &mut TimelineState, selected_date: NaiveDate) {
    if let Some(idx) = state.days.iter().position(|day| day.date == selected_date) {
        state.selected_day = idx;
    } else {
        state.selected_day = state.selected_day.min(state.days.len().saturating_sub(1));
    }
}

fn prepend_loading_days(state: &mut TimelineState, count: usize) -> NaiveDate {
    let oldest = state
        .days
        .first()
        .map(|day| day.date)
        .unwrap_or_else(|| Local::now().date_naive());
    let start = oldest - Duration::days(count as i64);
    let mut older = loading_days(start, count);
    older.append(&mut state.days);
    state.days = older;
    state.selected_day += count;
    state.viewport_top += count;
    start
}

fn ensure_visible_days_requested(state: &mut TimelineState, visible_count: usize, loader: &Loader) {
    if state.days.len() < visible_count {
        let needed = visible_count - state.days.len();
        let batches = needed.div_ceil(BATCH_DAY_COUNT);
        prepend_loading_days(state, batches * BATCH_DAY_COUNT);
    }
    request_oldest_loading_batch(state, loader);
}

fn ensure_older_days_requested(state: &mut TimelineState, _visible_count: usize, loader: &Loader) {
    prepend_loading_days(state, BATCH_DAY_COUNT);
    state.selected_day = BATCH_DAY_COUNT
        .saturating_sub(1)
        .min(state.days.len().saturating_sub(1));
    state.viewport_top = 0;
    request_oldest_loading_batch(state, loader);
}

fn request_oldest_loading_batch(state: &mut TimelineState, loader: &Loader) {
    if state.loading.older_batch_in_flight {
        return;
    }
    let Some(start) = state
        .days
        .iter()
        .find(|day| day.load_status == DayLoadStatus::Loading)
        .map(|day| day.date)
    else {
        return;
    };
    state.loading.oldest_requested = start;
    state.loading.older_batch_in_flight = true;
    let request = LoaderRequest::LoadRange {
        generation: state.generation,
        start,
        day_count: BATCH_DAY_COUNT,
    };
    if loader.requests.send(request).is_err() {
        state.loading.older_batch_in_flight = false;
        state.loading.last_error = Some("background loader stopped".into());
    }
}

fn apply_loader_event(state: &mut TimelineState, event: LoaderEvent) -> bool {
    match event {
        LoaderEvent::RangeLoaded {
            generation, days, ..
        } => {
            if generation != state.generation {
                return false;
            }
            let selected_date = state.selected().date;
            merge_days(&mut state.days, days);
            preserve_selected_date(state, selected_date);
            state.loading.older_batch_in_flight = false;
            state.loading.last_error = None;
            recompute_bounds(state);
            true
        }
        LoaderEvent::RangeFailed {
            generation,
            start,
            day_count,
            message,
        } => {
            if generation != state.generation {
                return false;
            }
            mark_failed_days(state, start, day_count);
            state.loading.older_batch_in_flight = false;
            state.loading.last_error = Some(truncate_to(&message, 80));
            true
        }
    }
}

fn mark_failed_days(state: &mut TimelineState, start: NaiveDate, day_count: usize) {
    for offset in 0..day_count {
        let date = start + Duration::days(offset as i64);
        if let Some(day) = state.days.iter_mut().find(|day| day.date == date) {
            day.load_status = DayLoadStatus::Failed;
            day.entries.clear();
            day.total_minutes = 0;
        }
    }
}

fn recompute_bounds(state: &mut TimelineState) {
    let today = state.today_local();
    let (start, end) = compute_global_bounds(&state.days, state.now, today);
    state.start_minute = start;
    state.end_minute = end;
    state.cursor_minute = state
        .cursor_minute
        .clamp(state.start_minute, state.end_minute);
}

fn compute_global_bounds(
    days: &[DayView],
    now: DateTime<Utc>,
    today_local: NaiveDate,
) -> (i64, i64) {
    let mut start = DEFAULT_DAY_START_HOUR * 60;
    let mut end = DEFAULT_DAY_END_HOUR * 60;
    for day in days {
        for entry in &day.entries {
            if entry.start_minute < start {
                start = entry.start_minute;
            }
            if entry.end_minute > end {
                end = entry.end_minute;
            }
        }
    }
    let now_local = now.with_timezone(&Local);
    if now_local.date_naive() == today_local {
        let now_minute = i64::from(now_local.hour()) * 60 + i64::from(now_local.minute());
        if now_minute > end {
            end = now_minute;
        }
    }
    if start < 0 {
        start = 0;
    }
    if end > 24 * 60 {
        end = 24 * 60;
    }
    if end <= start {
        end = start + 60;
    }
    (start, end)
}

fn local_at(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(date.year(), date.month(), date.day(), hour, minute, 0)
        .single()
        .unwrap_or_else(Local::now)
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>, CfdError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| CfdError::message(format!("invalid timestamp: {value}")))
}

fn draw(
    out: &mut Stdout,
    state: &TimelineState,
    cols: usize,
    rows: usize,
    visible_count: usize,
    clear_screen: bool,
    step: i64,
) -> Result<(), CfdError> {
    if clear_screen {
        queue!(out, Clear(ClearType::All)).map_err(io_err)?;
    }
    queue!(out, term_cursor::MoveTo(0, 0)).map_err(io_err)?;

    let min_needed: usize = 1 + ROWS_PER_DAY + 1 + 1 + 1 + LEGEND_HEIGHT as usize + 1;
    if cols < MIN_TERMINAL_WIDTH || rows < min_needed {
        clear_row(out, 0, cols)?;
        queue!(
            out,
            Print(format!(
                "Terminal too small ({cols}x{rows}). Need at least {MIN_TERMINAL_WIDTH}x{min_needed}."
            ))
        )
        .map_err(io_err)?;
        out.flush().map_err(io_err)?;
        return Ok(());
    }

    clear_row(out, 0, cols)?;
    let shortcuts = build_shortcuts(state, cols, step);
    draw_shortcut_bar(out, 0, cols, &shortcuts.top, SHORTCUT_BAR_COLOR)?;

    let viewport_end = (state.viewport_top + visible_count).min(state.days.len());
    let visible_days: Vec<(usize, &DayView)> = state
        .days
        .iter()
        .enumerate()
        .skip(state.viewport_top)
        .take(viewport_end - state.viewport_top)
        .collect();
    let date_labels: Vec<String> = visible_days
        .iter()
        .map(|(_, day)| day.date.format("%a %Y-%m-%d").to_string())
        .collect();
    let total_labels: Vec<String> = visible_days
        .iter()
        .map(|(_, day)| day_total_label(state, day))
        .collect();
    let date_width = date_labels
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0);
    let total_width = total_labels
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0);
    let marker_width = 2;
    let margin = 2usize;
    let left_margin = marker_width + date_width + margin;
    let right_margin = total_width + marker_width + margin;
    let bar_min = MIN_TERMINAL_WIDTH / 2;
    if cols <= left_margin + right_margin + bar_min {
        clear_row(out, 2, cols)?;
        queue!(
            out,
            term_cursor::MoveTo(0, 2),
            Print("Terminal too narrow for timeline.")
        )
        .map_err(io_err)?;
        out.flush().map_err(io_err)?;
        return Ok(());
    }
    let bar_offset = left_margin;
    let bar_cols = cols - left_margin - right_margin;
    let bar_offset_u16 = u16::try_from(bar_offset).unwrap_or(0);

    let first_day_row: u16 = 2;
    for (visible_idx, (day_idx, _day)) in visible_days.iter().enumerate() {
        let day_row = first_day_row + (visible_idx * ROWS_PER_DAY) as u16;
        let bar_row = day_row + 1;
        let is_selected = *day_idx == state.selected_day;
        let i = visible_idx;
        let day_idx = *day_idx;
        let marker = if is_selected { "▶ " } else { "  " };
        let row_background = day_row_background(state, day_idx);
        let row_foreground = day_row_foreground(state, day_idx);
        clear_row_with_background(out, day_row, cols, row_background)?;
        clear_row_with_background(out, bar_row, cols, row_background)?;
        queue!(out, term_cursor::MoveTo(0, day_row)).map_err(io_err)?;
        print_row_text(out, marker, row_background, row_foreground, false)?;
        print_row_text(
            out,
            &format!("{:<date_width$}", date_labels[i]),
            row_background,
            row_foreground,
            is_selected,
        )?;

        let total_chars = total_labels[i].chars().count();
        let total_col = u16::try_from(cols - marker_width - total_chars).unwrap_or(0);
        queue!(out, term_cursor::MoveTo(total_col, day_row)).map_err(io_err)?;
        print_row_text(
            out,
            &total_labels[i],
            row_background,
            row_foreground,
            is_selected,
        )?;
        let right_marker = if is_selected { " ◀" } else { "  " };
        print_row_text(out, right_marker, row_background, row_foreground, false)?;

        for block in compute_blocks(state, day_idx, bar_cols) {
            draw_block(out, &block, block.label.as_deref(), day_row, bar_offset)?;
            draw_block(
                out,
                &block,
                block.duration_label.as_deref(),
                bar_row,
                bar_offset,
            )?;
        }
    }

    let axis_row = first_day_row + (visible_days.len() * ROWS_PER_DAY) as u16;
    let labels_row = axis_row + 1;
    let (axis, labels) = render_axis(state, bar_cols);
    clear_row(out, axis_row, cols)?;
    clear_row(out, labels_row, cols)?;
    queue!(
        out,
        term_cursor::MoveTo(bar_offset_u16, axis_row),
        Print(&axis),
        term_cursor::MoveTo(bar_offset_u16, labels_row),
        Print(&labels)
    )
    .map_err(io_err)?;

    let cursor_col = col_at_minute(state.cursor_minute, state, bar_cols);
    let cursor_col_u16 =
        u16::try_from(bar_offset + cursor_col.min(bar_cols.saturating_sub(1))).unwrap_or(0);
    for i in 0..visible_days.len() {
        let first_row = first_day_row + (i * ROWS_PER_DAY) as u16;
        for row_offset in 0..ROWS_PER_DAY {
            let row = first_row + row_offset as u16;
            queue!(
                out,
                term_cursor::MoveTo(cursor_col_u16, row),
                SetAttribute(Attribute::Reverse),
                Print('│'),
                SetAttribute(Attribute::Reset)
            )
            .map_err(io_err)?;
        }
    }

    let legend_top: u16 = labels_row + 2;
    let legend = render_legend(state, cols);
    for i in 0..LEGEND_HEIGHT {
        clear_row(out, legend_top + i, cols)?;
    }
    for (i, line) in legend.iter().enumerate() {
        if i as u16 >= LEGEND_HEIGHT {
            break;
        }
        queue!(
            out,
            term_cursor::MoveTo(0, legend_top + i as u16),
            Print(line)
        )
        .map_err(io_err)?;
    }

    let shortcut_row = u16::try_from(rows.saturating_sub(1)).unwrap_or(0);
    let bottom_shortcuts = shortcuts.bottom;
    if bottom_shortcuts.is_empty() {
        clear_row(out, shortcut_row, cols)?;
    } else {
        draw_shortcut_bar(
            out,
            shortcut_row,
            cols,
            &bottom_shortcuts,
            bottom_shortcut_color(state),
        )?;
    }

    out.flush().map_err(io_err)?;
    Ok(())
}

fn clear_row(out: &mut Stdout, row: u16, cols: usize) -> Result<(), CfdError> {
    clear_row_with_background(out, row, cols, None)
}

fn clear_row_with_background(
    out: &mut Stdout,
    row: u16,
    cols: usize,
    background: Option<Color>,
) -> Result<(), CfdError> {
    queue!(out, ResetColor, SetAttribute(Attribute::Reset)).map_err(io_err)?;
    if let Some(background) = background {
        queue!(out, SetBackgroundColor(background)).map_err(io_err)?;
    }
    queue!(
        out,
        term_cursor::MoveTo(0, row),
        Print(" ".repeat(cols)),
        ResetColor,
        SetAttribute(Attribute::Reset),
        term_cursor::MoveTo(0, row)
    )
    .map_err(io_err)
}

fn print_row_text(
    out: &mut Stdout,
    text: &str,
    background: Option<Color>,
    foreground: Option<Color>,
    bold: bool,
) -> Result<(), CfdError> {
    queue!(out, ResetColor, SetAttribute(Attribute::Reset)).map_err(io_err)?;
    if let Some(background) = background {
        queue!(out, SetBackgroundColor(background)).map_err(io_err)?;
    }
    if let Some(foreground) = foreground {
        queue!(out, SetForegroundColor(foreground)).map_err(io_err)?;
    }
    if bold {
        queue!(out, SetAttribute(Attribute::Bold)).map_err(io_err)?;
    }
    queue!(out, Print(text), ResetColor, SetAttribute(Attribute::Reset)).map_err(io_err)
}

fn draw_shortcut_bar(
    out: &mut Stdout,
    row: u16,
    cols: usize,
    segments: &[ShortcutSegment],
    background: Color,
) -> Result<(), CfdError> {
    let text = shortcut_bar_text(segments, cols);
    queue!(
        out,
        term_cursor::MoveTo(0, row),
        SetBackgroundColor(background),
        SetForegroundColor(Color::Black),
        Print(format!("{text:<cols$}")),
        ResetColor,
        SetAttribute(Attribute::Reset),
        term_cursor::MoveTo(0, row)
    )
    .map_err(io_err)
}

fn shortcut_bar_text(segments: &[ShortcutSegment], cols: usize) -> String {
    let mut out = String::new();
    for (idx, segment) in segments.iter().enumerate() {
        let text = shortcut_segment_text(segment);
        let next_len = out.chars().count() + text.chars().count() + usize::from(idx > 0);
        if next_len > cols {
            break;
        }
        if idx > 0 {
            out.push('│');
        }
        out.push_str(&text);
    }
    out
}

fn rendered_shortcuts(segments: Vec<ShortcutSegment>, cols: usize) -> Vec<ShortcutSegment> {
    let mut rendered = Vec::new();
    let mut width = 0usize;
    for segment in segments {
        let segment_width = shortcut_segment_text(&segment).chars().count();
        let next_width = width + segment_width + usize::from(!rendered.is_empty());
        if next_width > cols {
            break;
        }
        width = next_width;
        rendered.push(segment);
    }
    rendered
}

fn shortcut_segment_text(segment: &ShortcutSegment) -> String {
    format!(" {}  {} ", segment.key, segment.label)
}

fn build_shortcuts(state: &TimelineState, cols: usize, step: i64) -> ShortcutSet {
    let top = rendered_shortcuts(raw_top_shortcuts(state), cols);
    let bottom = rendered_shortcuts(raw_bottom_shortcuts(state, step), cols);
    let mut accepted = top.clone();
    accepted.extend(bottom.iter().copied());
    ShortcutSet {
        top,
        bottom,
        accepted,
    }
}

fn global_shortcuts(state: &TimelineState, cols: usize) -> Vec<ShortcutSegment> {
    rendered_shortcuts(raw_top_shortcuts(state), cols)
}

fn raw_top_shortcuts(state: &TimelineState) -> Vec<ShortcutSegment> {
    if state.interaction.is_confirming() {
        vec![ShortcutSegment {
            key: "Ctrl-C",
            label: "quit",
            action: Action::Quit,
        }]
    } else if state.edit.is_some() {
        vec![
            ShortcutSegment {
                key: "←/→",
                label: "adjust",
                action: Action::EditStepLeft,
            },
            ShortcutSegment {
                key: "Enter",
                label: "save",
                action: Action::CommitEdit,
            },
            ShortcutSegment {
                key: "Esc",
                label: "cancel",
                action: Action::CancelEdit,
            },
            ShortcutSegment {
                key: "Ctrl-C",
                label: "quit",
                action: Action::Quit,
            },
        ]
    } else {
        let mut segments = vec![
            ShortcutSegment {
                key: "←/→",
                label: "move",
                action: Action::MoveCursorLeft,
            },
            ShortcutSegment {
                key: "↑/↓",
                label: "day",
                action: Action::SelectPreviousDay,
            },
            ShortcutSegment {
                key: "Home/End",
                label: "bounds",
                action: Action::JumpStart,
            },
            ShortcutSegment {
                key: "t",
                label: "now",
                action: Action::JumpNow,
            },
            ShortcutSegment {
                key: "r",
                label: "reload",
                action: Action::Reload,
            },
        ];
        if state.has_running_timer() {
            segments.push(ShortcutSegment {
                key: "p",
                label: "stop timer",
                action: Action::StopCurrentTimer,
            });
        }
        segments.push(ShortcutSegment {
            key: "q/Esc/Ctrl-C",
            label: "quit",
            action: Action::Quit,
        });
        segments
    }
}

fn bottom_shortcuts(state: &TimelineState, cols: usize, step: i64) -> Vec<ShortcutSegment> {
    build_shortcuts(state, cols, step).bottom
}

fn raw_bottom_shortcuts(state: &TimelineState, step: i64) -> Vec<ShortcutSegment> {
    if state.interaction.is_confirming() {
        raw_confirmation_shortcuts()
    } else if state.edit.is_some() {
        Vec::new()
    } else {
        raw_entry_shortcuts(state, step)
    }
}

fn entry_shortcuts(state: &TimelineState, cols: usize, step: i64) -> Vec<ShortcutSegment> {
    rendered_shortcuts(raw_entry_shortcuts(state, step), cols)
}

fn raw_entry_shortcuts(state: &TimelineState, step: i64) -> Vec<ShortcutSegment> {
    let Some(entry) = entry_at_cursor(state) else {
        return Vec::new();
    };
    if state.selected().load_status != DayLoadStatus::Loaded {
        return Vec::new();
    }

    let mut segments = Vec::new();
    if entry.running {
        if edit_action_available(state, entry, EditMode::Start, step) {
            segments.push(ShortcutSegment {
                key: "a",
                label: "move start",
                action: Action::BeginAdjustStart,
            });
        }
        return segments;
    }
    if edit_action_available(state, entry, EditMode::Move, step) {
        segments.push(ShortcutSegment {
            key: "m",
            label: "move",
            action: Action::BeginMoveEntry,
        });
    }
    if edit_action_available(state, entry, EditMode::Start, step) {
        segments.push(ShortcutSegment {
            key: "a",
            label: "move start",
            action: Action::BeginAdjustStart,
        });
    }
    if edit_action_available(state, entry, EditMode::End, step) {
        segments.push(ShortcutSegment {
            key: "e",
            label: "move end",
            action: Action::BeginAdjustEnd,
        });
    }
    if entry_can_split_at_cursor(entry, state.cursor_minute, step) {
        segments.push(ShortcutSegment {
            key: "s",
            label: "split",
            action: Action::SplitCursorEntry,
        });
    }
    if !state.has_running_timer() && entry.project_id.is_some() {
        segments.push(ShortcutSegment {
            key: "n",
            label: "start",
            action: Action::StartTimerFromCursorEntry,
        });
    }
    segments.push(ShortcutSegment {
        key: "d",
        label: "delete",
        action: Action::DeleteCursorEntry,
    });
    segments
}

fn entry_can_split_at_cursor(entry: &EntryView, cursor_minute: i64, step: i64) -> bool {
    let step = step.max(1);
    cursor_minute >= entry.start_minute + step && cursor_minute <= entry.end_minute - step
}

fn confirmation_shortcuts(_state: &TimelineState, cols: usize) -> Vec<ShortcutSegment> {
    rendered_shortcuts(raw_confirmation_shortcuts(), cols)
}

fn raw_confirmation_shortcuts() -> Vec<ShortcutSegment> {
    vec![
        ShortcutSegment {
            key: "y/Enter",
            label: "confirm",
            action: Action::ConfirmYes,
        },
        ShortcutSegment {
            key: "n/Esc",
            label: "cancel",
            action: Action::ConfirmNo,
        },
    ]
}

fn bottom_shortcut_color(_state: &TimelineState) -> Color {
    CURSOR_ENTRY_HIGHLIGHT_COLOR
}

fn day_row_background(state: &TimelineState, day_idx: usize) -> Option<Color> {
    if day_idx == state.selected_day {
        Some(CURSOR_ROW_BACKGROUND)
    } else {
        None
    }
}

fn day_row_foreground(state: &TimelineState, day_idx: usize) -> Option<Color> {
    if day_idx == state.selected_day {
        Some(CURSOR_ROW_FOREGROUND)
    } else {
        None
    }
}

fn day_total_label(state: &TimelineState, day: &DayView) -> String {
    match day.load_status {
        DayLoadStatus::Loading => "loading".into(),
        DayLoadStatus::Failed => "error".into(),
        DayLoadStatus::Loaded => {
            let total = day
                .entries
                .iter()
                .map(|entry| {
                    let (start, end) = entry_display_interval_for_day(state, day.date, entry);
                    (end - start).max(0)
                })
                .sum();
            format_duration_minutes(total)
        }
    }
}

fn header_status(state: &TimelineState) -> String {
    if let Some(error) = state.loading.last_error.as_deref() {
        return format!("load failed: {error}");
    }
    if state.loading.older_batch_in_flight
        || state
            .days
            .iter()
            .any(|day| day.load_status == DayLoadStatus::Loading)
    {
        return "loading older days...".into();
    }
    String::new()
}

#[derive(Debug, Clone)]
struct BlockRender {
    start_col: usize,
    width: usize,
    shade: char,
    color: Color,
    label: Option<String>,
    duration_label: Option<String>,
    running: bool,
    highlight: BlockHighlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockHighlight {
    None,
    Cursor,
    EditMove,
    EditStart,
    EditEnd,
}

const ENTRY_PALETTE: &[Color] = &[
    Color::Cyan,
    Color::Green,
    Color::Blue,
    Color::Magenta,
    Color::Red,
    Color::DarkCyan,
    Color::DarkGreen,
    Color::DarkBlue,
    Color::DarkMagenta,
    Color::DarkRed,
];

fn entry_color(entry: &EntryView) -> Color {
    let key = entry
        .task_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(entry.description.as_str());
    if key.is_empty() {
        return ENTRY_PALETTE[0];
    }
    let hash = fnv1a64(key);
    ENTRY_PALETTE[(hash as usize) % ENTRY_PALETTE.len()]
}

fn fnv1a64(value: &str) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn compute_blocks(state: &TimelineState, day_idx: usize, cols: usize) -> Vec<BlockRender> {
    let day = &state.days[day_idx];
    let is_selected = day_idx == state.selected_day;
    let cursor_idx = if is_selected {
        day.entries.iter().position(|entry| {
            let (start, end) = entry_display_interval_for_day(state, day.date, entry);
            state.cursor_minute >= start && state.cursor_minute < end.max(start + 1)
        })
    } else {
        None
    };

    day.entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            let (start_minute, end_minute, highlight) =
                block_interval_and_highlight(state, day, entry, cursor_idx == Some(idx));
            let start_col = col_at_minute(start_minute, state, cols);
            let end_col = col_at_minute(end_minute, state, cols)
                .max(start_col + 1)
                .min(cols);
            let width = end_col.saturating_sub(start_col);
            if width == 0 {
                return None;
            }
            let label = block_label(entry, width);
            let duration_label = block_duration_label(start_minute, end_minute, width);
            Some(BlockRender {
                start_col,
                width,
                shade: '█',
                color: entry_color(entry),
                label,
                duration_label,
                running: entry.running,
                highlight,
            })
        })
        .collect()
}

fn block_interval_and_highlight(
    state: &TimelineState,
    day: &DayView,
    entry: &EntryView,
    cursor_highlighted: bool,
) -> (i64, i64, BlockHighlight) {
    if let Some(edit) = &state.edit {
        if edit.date == day.date && edit.entry_id == entry.id {
            let highlight = match edit.mode {
                EditMode::Move => BlockHighlight::EditMove,
                EditMode::Start => BlockHighlight::EditStart,
                EditMode::End => BlockHighlight::EditEnd,
            };
            return (edit.draft_start_minute, edit.draft_end_minute, highlight);
        }
    }
    let (start, end) = entry_display_interval_for_day(state, day.date, entry);
    let highlight = if cursor_highlighted {
        BlockHighlight::Cursor
    } else {
        BlockHighlight::None
    };
    (start, end, highlight)
}

fn block_label(entry: &EntryView, width: usize) -> Option<String> {
    let min_block_width = LABEL_INSET + 2 * LABEL_PAD + 2;
    if width < min_block_width {
        return None;
    }
    let source = entry
        .task_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .or({
            if entry.description.is_empty() {
                None
            } else {
                Some(entry.description.as_str())
            }
        })?;
    let max_label = width - LABEL_INSET - 2 * LABEL_PAD - 1;
    Some(truncate_to(source, max_label))
}

fn block_duration_label(start_minute: i64, end_minute: i64, width: usize) -> Option<String> {
    let duration = format_duration_minutes((end_minute - start_minute).max(0));
    block_text_label(&duration, width)
}

fn block_text_label(source: &str, width: usize) -> Option<String> {
    let min_block_width = LABEL_INSET + 2 * LABEL_PAD + 2;
    if width < min_block_width || source.is_empty() {
        return None;
    }
    let max_label = width - LABEL_INSET - 2 * LABEL_PAD - 1;
    Some(truncate_to(source, max_label))
}

fn block_layout(block: &BlockRender, label: Option<&str>) -> (usize, usize, usize) {
    let label_chars = label.map(|value| value.chars().count()).unwrap_or(0);
    if label_chars == 0 {
        return (0, 0, block.width);
    }
    let segment = label_chars + 2 * LABEL_PAD;
    if block.width < LABEL_INSET + segment + 1 {
        return (0, 0, block.width);
    }
    let left = LABEL_INSET;
    let right = block.width - left - segment;
    (left, segment, right)
}

fn draw_block(
    out: &mut Stdout,
    block: &BlockRender,
    label: Option<&str>,
    bar_row: u16,
    bar_offset: usize,
) -> Result<(), CfdError> {
    if block.width == 0 {
        return Ok(());
    }
    let col = u16::try_from(bar_offset + block.start_col).unwrap_or(0);
    queue!(out, term_cursor::MoveTo(col, bar_row)).map_err(io_err)?;

    let (left, segment, right) = block_layout(block, label);

    if segment == 0 {
        emit_shade(out, block, block.width, block.running)?;
        emit_edit_edge(out, block, bar_row, bar_offset)?;
        return Ok(());
    }

    emit_shade(out, block, left, false)?;

    let label = label.unwrap_or("");
    let segment_text = format!(
        "{pad}{label}{pad}",
        pad = " ".repeat(LABEL_PAD),
        label = label
    );
    let label_color = if block_whole_highlighted(block) {
        CURSOR_ENTRY_HIGHLIGHT_COLOR
    } else {
        block.color
    };
    queue!(
        out,
        SetForegroundColor(label_color),
        SetAttribute(Attribute::Reverse),
        Print(segment_text),
        SetAttribute(Attribute::Reset),
        ResetColor
    )
    .map_err(io_err)?;

    emit_shade(out, block, right, block.running)?;
    emit_edit_edge(out, block, bar_row, bar_offset)?;
    Ok(())
}

fn block_whole_highlighted(block: &BlockRender) -> bool {
    matches!(
        block.highlight,
        BlockHighlight::Cursor | BlockHighlight::EditMove
    )
}

fn emit_edit_edge(
    out: &mut Stdout,
    block: &BlockRender,
    bar_row: u16,
    bar_offset: usize,
) -> Result<(), CfdError> {
    let Some(edge_offset) = (match block.highlight {
        BlockHighlight::EditStart => Some(0),
        BlockHighlight::EditEnd => Some(block.width.saturating_sub(1)),
        _ => None,
    }) else {
        return Ok(());
    };
    let col = u16::try_from(bar_offset + block.start_col + edge_offset).unwrap_or(0);
    queue!(
        out,
        term_cursor::MoveTo(col, bar_row),
        SetForegroundColor(CURSOR_ENTRY_HIGHLIGHT_COLOR),
        SetAttribute(Attribute::Bold),
        SetAttribute(Attribute::Reverse),
        Print(block.shade),
        SetAttribute(Attribute::Reset),
        ResetColor
    )
    .map_err(io_err)?;
    Ok(())
}

fn emit_shade(
    out: &mut Stdout,
    block: &BlockRender,
    count: usize,
    running_tail: bool,
) -> Result<(), CfdError> {
    if count == 0 {
        return Ok(());
    }
    if block_whole_highlighted(block) {
        queue!(
            out,
            SetForegroundColor(CURSOR_ENTRY_HIGHLIGHT_COLOR),
            SetAttribute(Attribute::Bold)
        )
        .map_err(io_err)?;
    } else {
        queue!(out, SetForegroundColor(block.color)).map_err(io_err)?;
    }
    let mut text: String = std::iter::repeat_n(block.shade, count).collect();
    if running_tail {
        text.pop();
        text.push('╌');
    }
    queue!(out, Print(text), SetAttribute(Attribute::Reset), ResetColor).map_err(io_err)?;
    Ok(())
}

fn render_axis(state: &TimelineState, cols: usize) -> (String, String) {
    let mut axis: Vec<char> = vec!['─'; cols];
    let mut labels: Vec<char> = vec![' '; cols];

    let start_hour = (state.start_minute / 60) as i32;
    let end_hour = ((state.end_minute + 59) / 60) as i32;

    for hour in start_hour..=end_hour {
        let minute = i64::from(hour) * 60;
        if minute < state.start_minute || minute > state.end_minute {
            continue;
        }
        let col = col_at_minute(minute, state, cols);
        if col < cols {
            axis[col] = '┬';
        }
        let label = format!("{:02}:00", hour.rem_euclid(24));
        let label_start = col.saturating_sub(2);
        if label_start + label.len() <= cols {
            for (i, ch) in label.chars().enumerate() {
                let target = label_start + i;
                if labels[target] == ' ' {
                    labels[target] = ch;
                }
            }
        }
    }

    if !axis.is_empty() {
        axis[0] = '└';
        axis[cols - 1] = '┘';
    }

    (axis.iter().collect(), labels.iter().collect())
}

fn col_at_minute(minute: i64, state: &TimelineState, cols: usize) -> usize {
    let span = (state.end_minute - state.start_minute).max(1);
    let clamped = minute.clamp(state.start_minute, state.end_minute);
    let offset = clamped - state.start_minute;
    let col = (offset as i128 * cols as i128 / span as i128) as usize;
    col.min(cols.saturating_sub(1))
}

fn render_legend(state: &TimelineState, cols: usize) -> Vec<String> {
    let entry = entry_at_cursor(state);
    let cursor_label = minute_to_label(state.cursor_minute);
    let day_label = state.selected().date.format("%a %Y-%m-%d").to_string();
    let mut lines = Vec::new();

    let header = format!("Cursor: {cursor_label} on {day_label}");
    lines.push(header);
    lines.push("─".repeat(cols));
    let status = header_status(state);
    if !status.is_empty() {
        lines.push(truncate_to(&status, cols));
    }

    match entry {
        Some(entry) => {
            let (display_start, display_end) = entry_display_interval(state, entry);
            let duration_minutes = (display_end - display_start).max(0);
            let time_label = format!(
                "{}–{}{}",
                minute_to_label(display_start),
                minute_to_label(display_end),
                if entry.running { " (running)" } else { "" }
            );
            let duration_label = format_duration_minutes(duration_minutes);
            let project = entry.project_name.clone().unwrap_or_else(|| "—".into());
            let task = entry.task_id.clone().unwrap_or_else(|| "—".into());
            let tags = if entry.tag_ids.is_empty() {
                "—".into()
            } else {
                entry.tag_ids.join(", ")
            };
            let description = if entry.description.is_empty() {
                "—".into()
            } else {
                entry.description.clone()
            };

            lines.push(two_columns(("ID", &entry.id), ("Time", &time_label), cols));
            lines.push(two_columns(
                ("Project", &project),
                ("Duration", &duration_label),
                cols,
            ));
            lines.push(two_columns(("Task", &task), ("Tags", &tags), cols));
            lines.push(single_column("Description", &description, cols));
        }
        None => {
            lines.push(format!("(no entry — gap at {cursor_label})"));
            lines.push(String::new());
            lines.push(String::new());
            lines.push(String::new());
            lines.push(String::new());
        }
    }

    if let Some(edit_line) = edit_status_line(state) {
        if lines.len() >= LEGEND_HEIGHT as usize {
            lines.truncate(LEGEND_HEIGHT as usize - 1);
        }
        lines.push(truncate_to(&edit_line, cols));
    }

    if let Some(line) = state.interaction.line() {
        if lines.len() >= LEGEND_HEIGHT as usize {
            lines.truncate(LEGEND_HEIGHT as usize - 1);
        }
        lines.push(truncate_to(line, cols));
    }

    lines
}

fn entry_display_interval(state: &TimelineState, entry: &EntryView) -> (i64, i64) {
    entry_display_interval_for_day(state, state.selected().date, entry)
}

fn entry_display_interval_for_day(
    state: &TimelineState,
    date: NaiveDate,
    entry: &EntryView,
) -> (i64, i64) {
    if let Some(edit) = &state.edit {
        if edit.date == date && edit.entry_id == entry.id {
            return (
                edit.draft_start_minute,
                edit_effective_end_minute(state, edit),
            );
        }
    }
    if entry.running && date == state.now.with_timezone(&Local).date_naive() {
        return (
            entry.start_minute,
            state.now_minute().max(entry.start_minute),
        );
    }
    (entry.start_minute, entry.end_minute)
}

fn edit_status_line(state: &TimelineState) -> Option<String> {
    let edit = state.edit.as_ref()?;
    let mode = match edit.mode {
        EditMode::Move => "move",
        EditMode::Start => "start",
        EditMode::End => "end",
    };
    Some(format!(
        "Editing {mode}: {}-{}. Enter saves; Esc cancels.",
        minute_to_label(edit.draft_start_minute),
        minute_to_label(edit_effective_end_minute(state, edit))
    ))
}

fn entry_at_cursor(state: &TimelineState) -> Option<&EntryView> {
    let day = state.selected();
    if let Some(edit) = &state.edit {
        if edit.date == day.date {
            return day.entries.iter().find(|entry| entry.id == edit.entry_id);
        }
    }
    let minute = state.cursor_minute;
    day.entries.iter().find(|entry| {
        let (start, end) = entry_display_interval_for_day(state, day.date, entry);
        minute >= start && minute < end.max(start + 1)
    })
}

fn two_columns(left: (&str, &str), right: (&str, &str), cols: usize) -> String {
    let half = cols / 2;
    let left_text = format_field(left.0, left.1, half);
    let right_text = format_field(right.0, right.1, cols - half);
    format!("{left_text}{right_text}")
}

fn single_column(label: &str, value: &str, cols: usize) -> String {
    format_field(label, value, cols)
}

fn format_field(label: &str, value: &str, width: usize) -> String {
    let prefix = format!("{label}: ");
    if width <= prefix.len() {
        return prefix.chars().take(width).collect();
    }
    let avail = width - prefix.len();
    let truncated = truncate_to(value, avail);
    let padded = format!("{truncated:<avail$}");
    format!("{prefix}{padded}")
}

fn truncate_to(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else if width <= 1 {
        value.chars().take(width).collect()
    } else {
        let mut out: String = value.chars().take(width - 1).collect();
        out.push('…');
        out
    }
}

fn minute_to_label(minute: i64) -> String {
    let h = (minute.div_euclid(60)).rem_euclid(24);
    let m = minute.rem_euclid(60);
    format!("{h:02}:{m:02}")
}

fn format_duration_minutes(minutes: i64) -> String {
    let h = minutes / 60;
    let m = minutes % 60;
    if h > 0 && m > 0 {
        format!("{h}h {m}m")
    } else if h > 0 {
        format!("{h}h")
    } else {
        format!("{m}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{StoredSwitch, StoredTimerFields, TimeInterval};

    fn day_view(date: NaiveDate, entries: Vec<EntryView>) -> DayView {
        let total = entries
            .iter()
            .map(|e| (e.end_minute - e.start_minute).max(0))
            .sum();
        DayView {
            date,
            entries,
            total_minutes: total,
            load_status: DayLoadStatus::Loaded,
        }
    }

    fn loading_state(date: NaiveDate) -> LoadingState {
        LoadingState {
            oldest_requested: date,
            older_batch_in_flight: false,
            last_error: None,
        }
    }

    fn state_with(start_minute: i64, end_minute: i64, entries: Vec<EntryView>) -> TimelineState {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        TimelineState {
            days: vec![day_view(date, entries)],
            selected_day: 0,
            viewport_top: 0,
            start_minute,
            end_minute,
            cursor_minute: start_minute,
            now: Utc::now(),
            current_timer_id: None,
            loading: loading_state(date),
            interaction: InteractionState::default(),
            edit: None,
            generation: 0,
        }
    }

    fn multi_day_state(days: Vec<DayView>) -> TimelineState {
        let selected = days.len() - 1;
        let oldest = days.first().map(|day| day.date).unwrap();
        TimelineState {
            days,
            selected_day: selected,
            viewport_top: 0,
            start_minute: 8 * 60,
            end_minute: 18 * 60,
            cursor_minute: 9 * 60,
            now: Utc::now(),
            current_timer_id: None,
            loading: loading_state(oldest),
            interaction: InteractionState::default(),
            edit: None,
            generation: 0,
        }
    }

    fn view(id: &str, start: i64, end: i64) -> EntryView {
        EntryView {
            id: id.into(),
            description: "desc".into(),
            project_id: Some("p1".into()),
            project_name: Some("Proj".into()),
            task_id: Some("t1".into()),
            tag_ids: vec![],
            start_minute: start,
            end_minute: end,
            running: false,
        }
    }

    fn running_view(id: &str, start: i64, now: i64) -> EntryView {
        let mut entry = view(id, start, now);
        entry.running = true;
        entry
    }

    fn set_now(state: &mut TimelineState, hour: u32, minute: u32) {
        state.now = local_at(state.selected().date, hour, minute).with_timezone(&Utc);
    }

    fn test_loader() -> (Loader, Receiver<LoaderRequest>) {
        let (request_tx, request_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::channel();
        (
            Loader {
                requests: request_tx,
                events: event_rx,
            },
            request_rx,
        )
    }

    #[derive(Clone)]
    struct NoopTransport;

    impl HttpTransport for NoopTransport {
        fn get(&self, _url: &str, _api_key: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected get"))
        }

        fn post(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected post"))
        }

        fn put(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected put"))
        }

        fn patch(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected patch"))
        }

        fn delete(&self, _url: &str, _api_key: &str) -> Result<(), CfdError> {
            Err(CfdError::message("unexpected delete"))
        }
    }

    #[test]
    fn step_minutes_falls_back_to_15_when_rounding_off() {
        assert_eq!(step_minutes_from_rounding(RoundingMode::Off), 15);
        assert_eq!(step_minutes_from_rounding(RoundingMode::OneMinute), 1);
        assert_eq!(step_minutes_from_rounding(RoundingMode::FiveMinutes), 5);
        assert_eq!(step_minutes_from_rounding(RoundingMode::TenMinutes), 10);
        assert_eq!(step_minutes_from_rounding(RoundingMode::FifteenMinutes), 15);
    }

    #[test]
    fn visible_days_use_available_terminal_rows() {
        assert_eq!(compute_visible_days(20, usize::MAX), 3);
        assert_eq!(compute_visible_days(8, usize::MAX), 1);
        assert_eq!(compute_visible_days(20, 3), 3);
    }

    #[test]
    fn loading_days_end_at_expected_visible_range() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let days = loading_days(today - Duration::days(4), 5);
        assert_eq!(days.len(), 5);
        assert_eq!(days.first().unwrap().date, today - Duration::days(4));
        assert_eq!(days.last().unwrap().date, today);
        assert!(days
            .iter()
            .all(|day| day.load_status == DayLoadStatus::Loading));
    }

    #[test]
    fn col_at_minute_maps_endpoints() {
        let state = state_with(8 * 60, 18 * 60, vec![]);
        assert_eq!(col_at_minute(8 * 60, &state, 100), 0);
        assert_eq!(col_at_minute(18 * 60, &state, 100), 99);
    }

    #[test]
    fn snap_aligns_cursor_to_step_grid_relative_to_start() {
        let mut state = state_with(8 * 60, 18 * 60, vec![]);
        state.cursor_minute = 9 * 60 + 7;
        snap_cursor(&mut state, 15);
        assert_eq!(state.cursor_minute, 9 * 60);

        state.cursor_minute = 9 * 60 + 8;
        snap_cursor(&mut state, 15);
        assert_eq!(state.cursor_minute, 9 * 60 + 15);
    }

    #[test]
    fn cursor_clamps_to_bounds() {
        let mut state = state_with(8 * 60, 10 * 60, vec![]);
        state.cursor_minute = 9 * 60;
        state.move_cursor(-180);
        assert_eq!(state.cursor_minute, 8 * 60);
        state.move_cursor(180);
        assert_eq!(state.cursor_minute, 10 * 60);
    }

    #[test]
    fn entry_lookup_finds_block_in_selected_day() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let state = TimelineState {
            days: vec![
                day_view(date - Duration::days(1), vec![view("y", 540, 600)]),
                day_view(date, vec![view("a", 540, 600), view("b", 600, 660)]),
            ],
            selected_day: 1,
            viewport_top: 0,
            start_minute: 0,
            end_minute: 24 * 60,
            cursor_minute: 545,
            now: Utc::now(),
            current_timer_id: None,
            loading: loading_state(date - Duration::days(1)),
            interaction: InteractionState::default(),
            edit: None,
            generation: 0,
        };
        assert_eq!(entry_at_cursor(&state).map(|e| e.id.as_str()), Some("a"));
    }

    #[test]
    fn entry_view_includes_project_id() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let entry = TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: Some("t1".into()),
            tag_ids: vec!["tag1".into()],
            description: "Build".into(),
            time_interval: TimeInterval {
                start: local_at(date, 9, 0).to_rfc3339(),
                end: Some(local_at(date, 10, 0).to_rfc3339()),
                duration: None,
            },
        };
        let projects = BTreeMap::from([("p1".into(), "Project One".into())]);

        let view = entry_view_for_day(&entry, date, Utc::now(), &projects)
            .unwrap()
            .unwrap();

        assert_eq!(view.project_id.as_deref(), Some("p1"));
        assert_eq!(view.project_name.as_deref(), Some("Project One"));
    }

    #[test]
    fn compute_blocks_places_entries_at_correct_columns() {
        let state = state_with(8 * 60, 10 * 60, vec![view("a", 9 * 60, 9 * 60 + 30)]);
        let blocks = compute_blocks(&state, 0, 80);
        assert_eq!(blocks.len(), 1);
        let expected_start = col_at_minute(9 * 60, &state, 80);
        assert_eq!(blocks[0].start_col, expected_start);
        assert!(blocks[0].width > 0);
        assert_eq!(blocks[0].shade, '█');
        assert_eq!(blocks[0].duration_label.as_deref(), Some("30m"));
    }

    #[test]
    fn block_label_prefers_task_then_description() {
        let mut entry = view("a", 9 * 60, 9 * 60 + 60);
        entry.task_id = Some("DateField".into());
        entry.description = "Planning".into();
        assert_eq!(block_label(&entry, 30).as_deref(), Some("DateField"));

        entry.task_id = Some(String::new());
        assert_eq!(block_label(&entry, 30).as_deref(), Some("Planning"));

        entry.task_id = None;
        entry.description = String::new();
        assert!(block_label(&entry, 30).is_none());
    }

    #[test]
    fn block_label_omitted_when_too_narrow() {
        let mut entry = view("a", 0, 60);
        entry.task_id = Some("Task".into());
        assert!(block_label(&entry, 4).is_none());
    }

    #[test]
    fn cursor_block_highlighted_only_on_selected_day() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let state = TimelineState {
            days: vec![
                day_view(date - Duration::days(1), vec![view("y", 540, 600)]),
                day_view(date, vec![view("a", 540, 600)]),
            ],
            selected_day: 1,
            viewport_top: 0,
            start_minute: 8 * 60,
            end_minute: 18 * 60,
            cursor_minute: 9 * 60 + 10,
            now: Utc::now(),
            current_timer_id: None,
            loading: loading_state(date - Duration::days(1)),
            interaction: InteractionState::default(),
            edit: None,
            generation: 0,
        };
        let other = compute_blocks(&state, 0, 80);
        assert_eq!(other[0].highlight, BlockHighlight::None);
        let selected = compute_blocks(&state, 1, 80);
        assert_eq!(selected[0].highlight, BlockHighlight::Cursor);
    }

    #[test]
    fn day_bounds_extend_to_cover_entries_outside_default_window() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let days = vec![day_view(
            date,
            vec![view("a", 6 * 60, 7 * 60), view("b", 19 * 60, 21 * 60)],
        )];
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 4, 0, 0).unwrap();
        let (start, end) = compute_global_bounds(&days, now, date);
        assert_eq!(start, 6 * 60);
        assert_eq!(end, 21 * 60);
    }

    #[test]
    fn truncate_keeps_short_strings_and_appends_ellipsis() {
        assert_eq!(truncate_to("hello", 10), "hello");
        assert_eq!(truncate_to("hello world", 6), "hello…");
    }

    #[test]
    fn minute_to_label_formats_zero_padded() {
        assert_eq!(minute_to_label(8 * 60 + 5), "08:05");
        assert_eq!(minute_to_label(0), "00:00");
        assert_eq!(minute_to_label(13 * 60 + 30), "13:30");
    }

    #[test]
    fn entries_with_same_task_share_a_color() {
        let mut a = view("a", 0, 60);
        let mut b = view("b", 60, 120);
        a.task_id = Some("DateField".into());
        b.task_id = Some("DateField".into());
        let mut c = view("c", 120, 180);
        c.task_id = Some("Other".into());
        assert_eq!(entry_color(&a), entry_color(&b));
        assert_ne!(entry_color(&a), entry_color(&c));
    }

    #[test]
    fn entry_color_falls_back_to_description_when_no_task() {
        let mut a = view("a", 0, 60);
        let mut b = view("b", 60, 120);
        a.task_id = None;
        b.task_id = None;
        a.description = "Standup".into();
        b.description = "Standup".into();
        assert_eq!(entry_color(&a), entry_color(&b));
    }

    #[test]
    fn duration_minutes_formats_compactly() {
        assert_eq!(format_duration_minutes(45), "45m");
        assert_eq!(format_duration_minutes(60), "1h");
        assert_eq!(format_duration_minutes(90), "1h 30m");
    }

    #[test]
    fn global_shortcuts_include_navigation_reload_and_quit() {
        let state = state_with(8 * 60, 18 * 60, vec![]);
        let keys = global_shortcuts(&state, 120)
            .into_iter()
            .map(|segment| segment.key)
            .collect::<Vec<_>>();
        assert!(keys.contains(&"←/→"));
        assert!(keys.contains(&"↑/↓"));
        assert!(keys.contains(&"Home/End"));
        assert!(keys.contains(&"t"));
        assert!(keys.contains(&"r"));
        assert!(keys.contains(&"q/Esc/Ctrl-C"));
        assert!(!keys.contains(&"p"));
    }

    #[test]
    fn global_shortcuts_include_stop_only_when_timer_is_running() {
        let mut running = view("running", 9 * 60, 10 * 60);
        running.running = true;
        let mut running_state = state_with(8 * 60, 18 * 60, vec![running]);
        running_state.current_timer_id = Some("running".into());
        assert!(global_shortcuts(&running_state, 120)
            .iter()
            .any(|segment| segment.key == "p"));

        let idle_state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        assert!(!global_shortcuts(&idle_state, 120)
            .iter()
            .any(|segment| segment.key == "p"));
    }

    #[test]
    fn timer_shortcut_uses_current_timer_state_not_visible_entries() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("finished", 9 * 60, 10 * 60)]);
        state.current_timer_id = Some("timer-outside-visible-range".into());

        assert!(global_shortcuts(&state, 120)
            .iter()
            .any(|segment| segment.key == "p"));
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::StopCurrentTimer
        );
    }

    #[test]
    fn visible_running_entry_without_current_timer_does_not_enable_stop() {
        let mut running = view("running", 9 * 60, 10 * 60);
        running.running = true;
        let mut state = state_with(8 * 60, 18 * 60, vec![running]);
        state.cursor_minute = 9 * 60 + 15;

        assert!(!global_shortcuts(&state, 120)
            .iter()
            .any(|segment| segment.key == "p"));
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Beep
        );
    }

    #[test]
    fn running_entry_rendering_uses_current_minute() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut running = view("running", 9 * 60, 10 * 60);
        running.running = true;
        let mut state = state_with(8 * 60, 18 * 60, vec![running]);
        state.now = local_at(date, 10, 5).with_timezone(&Utc);
        state.cursor_minute = 9 * 60 + 30;

        let blocks = compute_blocks(&state, 0, 120);
        let legend = render_legend(&state, 120).join("\n");

        assert_eq!(blocks[0].duration_label.as_deref(), Some("1h 5m"));
        assert_eq!(day_total_label(&state, state.selected()), "1h 5m");
        assert!(legend.contains("Time: 09:00–10:05 (running)"));
        assert!(legend.contains("Duration: 1h 5m"));
    }

    #[test]
    fn recompute_bounds_extends_axis_for_running_timer_minute_tick() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut running = view("running", 17 * 60, 18 * 60);
        running.running = true;
        let mut state = state_with(8 * 60, 18 * 60, vec![running]);
        state.now = local_at(date, 18, 5).with_timezone(&Utc);

        recompute_bounds(&mut state);

        assert_eq!(state.end_minute, 18 * 60 + 5);
    }

    #[test]
    fn entry_shortcuts_require_loaded_entry_under_cursor() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        state.cursor_minute = 8 * 60 + 10;
        assert!(entry_shortcuts(&state, 120, 15).is_empty());

        state.cursor_minute = 9 * 60 + 15;
        assert_eq!(
            entry_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| segment.key)
                .collect::<Vec<_>>(),
            vec!["m", "a", "e", "s", "n", "d"]
        );

        state.days[0].load_status = DayLoadStatus::Loading;
        assert!(entry_shortcuts(&state, 120, 15).is_empty());
    }

    #[test]
    fn entry_shortcuts_keep_split_for_finished_entry_when_timer_is_running() {
        let mut running = view("running", 8 * 60, 8 * 60 + 30);
        running.running = true;
        let mut state = state_with(
            8 * 60,
            18 * 60,
            vec![running, view("finished", 9 * 60, 10 * 60)],
        );
        state.current_timer_id = Some("running".into());
        state.cursor_minute = 9 * 60 + 15;
        assert_eq!(
            entry_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| segment.key)
                .collect::<Vec<_>>(),
            vec!["m", "a", "e", "s", "d"]
        );
    }

    #[test]
    fn split_shortcut_requires_valid_cursor_split_position() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("entry", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60;
        assert!(!entry_shortcuts(&state, 120, 15)
            .iter()
            .any(|segment| segment.key == "s"));

        state.cursor_minute = 9 * 60 + 15;
        assert!(entry_shortcuts(&state, 120, 15)
            .iter()
            .any(|segment| segment.key == "s"));

        state.cursor_minute = 9 * 60 + 45;
        assert!(entry_shortcuts(&state, 120, 15)
            .iter()
            .any(|segment| segment.key == "s"));

        state.cursor_minute = 10 * 60;
        assert!(!entry_shortcuts(&state, 120, 15)
            .iter()
            .any(|segment| segment.key == "s"));
    }

    #[test]
    fn split_shortcut_requires_two_rounding_steps() {
        let mut short = state_with(8 * 60, 18 * 60, vec![view("short", 9 * 60, 9 * 60 + 29)]);
        short.cursor_minute = 9 * 60 + 10;
        assert_eq!(
            entry_shortcuts(&short, 120, 15)
                .into_iter()
                .map(|segment| segment.key)
                .collect::<Vec<_>>(),
            vec!["m", "a", "e", "n", "d"]
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
                &mut short,
                15,
                120,
            ),
            Action::Beep
        );

        let mut long_enough = state_with(8 * 60, 18 * 60, vec![view("long", 9 * 60, 9 * 60 + 30)]);
        long_enough.cursor_minute = 9 * 60 + 15;
        assert_eq!(
            entry_shortcuts(&long_enough, 120, 15)
                .into_iter()
                .map(|segment| segment.key)
                .collect::<Vec<_>>(),
            vec!["m", "a", "e", "s", "n", "d"]
        );

        let mut ten_minute = state_with(8 * 60, 18 * 60, vec![view("ten", 9 * 60, 9 * 60 + 10)]);
        ten_minute.cursor_minute = 9 * 60 + 5;
        assert!(entry_shortcuts(&ten_minute, 120, 5)
            .iter()
            .any(|segment| segment.key == "s"));
    }

    #[test]
    fn start_shortcut_requires_project_and_no_running_timer() {
        let mut no_project = view("no-project", 9 * 60, 10 * 60);
        no_project.project_id = None;
        let mut state = state_with(8 * 60, 18 * 60, vec![no_project]);
        state.cursor_minute = 9 * 60 + 15;
        assert_eq!(
            entry_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| segment.key)
                .collect::<Vec<_>>(),
            vec!["m", "a", "e", "s", "d"]
        );

        state.days[0].entries[0].project_id = Some("p1".into());
        state.current_timer_id = Some("running".into());
        assert_eq!(
            entry_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| segment.key)
                .collect::<Vec<_>>(),
            vec!["m", "a", "e", "s", "d"]
        );
    }

    #[test]
    fn context_shortcuts_hide_for_loading_failed_and_gap_but_show_running_start_edit() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        state.cursor_minute = 8 * 60 + 30;
        assert!(bottom_shortcuts(&state, 120, 15).is_empty());

        state.cursor_minute = 9 * 60 + 15;
        state.days[0].load_status = DayLoadStatus::Loading;
        assert!(bottom_shortcuts(&state, 120, 15).is_empty());

        state.days[0].load_status = DayLoadStatus::Failed;
        assert!(bottom_shortcuts(&state, 120, 15).is_empty());

        state.days[0].load_status = DayLoadStatus::Loaded;
        state.days[0].entries[0].running = true;
        set_now(&mut state, 10, 0);
        assert_eq!(
            bottom_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| (segment.key, segment.label))
                .collect::<Vec<_>>(),
            vec![("a", "move start")]
        );
    }

    #[test]
    fn entry_shortcuts_show_only_move_start_for_running_entry() {
        let mut running = view("running", 9 * 60, 10 * 60);
        running.running = true;
        let mut state = state_with(8 * 60, 18 * 60, vec![running]);
        set_now(&mut state, 10, 0);
        state.cursor_minute = 9 * 60 + 10;
        assert_eq!(
            entry_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| (segment.key, segment.label))
                .collect::<Vec<_>>(),
            vec![("a", "move start")]
        );
        assert_eq!(
            bottom_shortcuts(&state, 120, 15)
                .into_iter()
                .map(|segment| (segment.key, segment.label))
                .collect::<Vec<_>>(),
            vec![("a", "move start")]
        );
    }

    #[test]
    fn bottom_shortcut_color_uses_cursor_entry_highlight_color() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 10;
        assert_ne!(
            entry_color(entry_at_cursor(&state).unwrap()),
            CURSOR_ENTRY_HIGHLIGHT_COLOR
        );
        assert_eq!(bottom_shortcut_color(&state), CURSOR_ENTRY_HIGHLIGHT_COLOR);
    }

    #[test]
    fn day_row_background_marks_selected_day_only() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let state = TimelineState {
            days: vec![
                day_view(date - Duration::days(1), vec![view("y", 540, 600)]),
                day_view(date, vec![view("a", 540, 600)]),
            ],
            selected_day: 1,
            viewport_top: 0,
            start_minute: 8 * 60,
            end_minute: 18 * 60,
            cursor_minute: 9 * 60 + 10,
            now: Utc::now(),
            current_timer_id: None,
            loading: loading_state(date - Duration::days(1)),
            interaction: InteractionState::default(),
            edit: None,
            generation: 0,
        };

        assert_eq!(day_row_background(&state, 0), None);
        assert_eq!(day_row_background(&state, 1), Some(CURSOR_ROW_BACKGROUND));
    }

    #[test]
    fn day_row_foreground_marks_selected_day_white_only() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let state = TimelineState {
            days: vec![
                day_view(date - Duration::days(1), vec![view("y", 540, 600)]),
                day_view(date, vec![view("a", 540, 600)]),
            ],
            selected_day: 1,
            viewport_top: 0,
            start_minute: 8 * 60,
            end_minute: 18 * 60,
            cursor_minute: 9 * 60 + 10,
            now: Utc::now(),
            current_timer_id: None,
            loading: loading_state(date - Duration::days(1)),
            interaction: InteractionState::default(),
            edit: None,
            generation: 0,
        };

        assert_eq!(day_row_foreground(&state, 0), None);
        assert_eq!(day_row_foreground(&state, 1), Some(CURSOR_ROW_FOREGROUND));
    }

    #[test]
    fn top_shortcut_bar_uses_white_background() {
        assert_eq!(SHORTCUT_BAR_COLOR, Color::White);
    }

    #[test]
    fn legend_no_longer_contains_actions_line() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 10;
        let legend = render_legend(&state, 120).join("\n");
        assert!(!legend.contains("Actions:"));
    }

    #[test]
    fn shortcut_bar_truncates_to_complete_segments() {
        let state = state_with(8 * 60, 18 * 60, vec![]);
        assert_eq!(global_shortcuts(&state, 11).len(), 1);
        assert_eq!(global_shortcuts(&state, 10).len(), 0);
        assert_eq!(
            shortcut_bar_text(&global_shortcuts(&state, 11), 11),
            " ←/→  move "
        );
    }

    #[test]
    fn action_keys_map_when_idle() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::Redraw
        );
        assert_eq!(
            state.edit.as_ref().map(|edit| edit.mode),
            Some(EditMode::Move)
        );
        state.cancel_edit();
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::Redraw
        );
        assert_eq!(
            state.edit.as_ref().map(|edit| edit.mode),
            Some(EditMode::Start)
        );
        state.cancel_edit();
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::Redraw
        );
        assert_eq!(
            state.edit.as_ref().map(|edit| edit.mode),
            Some(EditMode::End)
        );
        state.cancel_edit();
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::StartTimerFromCursorEntry
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::SplitCursorEntry
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::DeleteCursorEntry
        );
    }

    #[test]
    fn edit_shortcuts_follow_candidate_validity() {
        let mut state = state_with(
            0,
            24 * 60,
            vec![
                view("left", 8 * 60, 9 * 60),
                view("edit", 9 * 60, 10 * 60),
                view("right", 10 * 60, 11 * 60),
            ],
        );
        state.cursor_minute = 9 * 60 + 15;

        let keys = entry_shortcuts(&state, 120, 15)
            .into_iter()
            .map(|segment| segment.key)
            .collect::<Vec<_>>();

        assert!(!keys.contains(&"m"), "move is blocked on both sides");
        assert!(keys.contains(&"a"), "start can still move inward");
        assert!(keys.contains(&"e"), "end can still move inward");
    }

    #[test]
    fn edit_shortcut_labels_use_move_start_and_move_end() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;

        let labels = entry_shortcuts(&state, 120, 15)
            .into_iter()
            .map(|segment| (segment.key, segment.label))
            .collect::<Vec<_>>();

        assert!(labels.contains(&("m", "move")));
        assert!(labels.contains(&("a", "move start")));
        assert!(labels.contains(&("e", "move end")));
    }

    #[test]
    fn running_entry_only_exposes_move_start_shortcut() {
        let mut state = state_with(
            8 * 60,
            18 * 60,
            vec![running_view("running", 9 * 60, 10 * 60)],
        );
        set_now(&mut state, 10, 0);
        state.current_timer_id = Some("running".into());
        state.cursor_minute = 9 * 60 + 15;

        let shortcuts = entry_shortcuts(&state, 120, 15)
            .into_iter()
            .map(|segment| (segment.key, segment.label))
            .collect::<Vec<_>>();

        assert_eq!(shortcuts, vec![("a", "move start")]);
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Redraw
        );
        assert_eq!(
            state.edit.as_ref().map(|edit| (edit.mode, edit.running)),
            Some((EditMode::Start, true))
        );
        state.cancel_edit();

        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Beep
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Beep
        );
    }

    #[test]
    fn edit_steps_update_move_start_and_end_drafts() {
        let mut move_state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        move_state.cursor_minute = 9 * 60 + 15;
        assert!(move_state.start_edit(EditMode::Move));
        assert!(move_state.apply_edit_step(15));
        let edit = move_state.edit.as_ref().unwrap();
        assert_eq!(edit.draft_start_minute, 9 * 60 + 15);
        assert_eq!(edit.draft_end_minute, 10 * 60 + 15);
        assert_eq!(move_state.cursor_minute, 9 * 60 + 30);

        let mut start_state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        start_state.cursor_minute = 9 * 60 + 15;
        assert!(start_state.start_edit(EditMode::Start));
        assert!(start_state.apply_edit_step(15));
        let edit = start_state.edit.as_ref().unwrap();
        assert_eq!(edit.draft_start_minute, 9 * 60 + 15);
        assert_eq!(edit.draft_end_minute, 10 * 60);
        assert_eq!(start_state.cursor_minute, 9 * 60 + 15);

        let mut end_state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        end_state.cursor_minute = 9 * 60 + 15;
        assert!(end_state.start_edit(EditMode::End));
        assert!(end_state.apply_edit_step(-15));
        let edit = end_state.edit.as_ref().unwrap();
        assert_eq!(edit.draft_start_minute, 9 * 60);
        assert_eq!(edit.draft_end_minute, 10 * 60 - 15);
        assert_eq!(end_state.cursor_minute, 10 * 60 - 15);
    }

    #[test]
    fn edit_steps_block_overlap_invalid_duration_and_day_bounds() {
        let mut overlap = state_with(
            8 * 60,
            18 * 60,
            vec![
                view("edit", 9 * 60, 10 * 60),
                view("neighbor", 10 * 60, 11 * 60),
            ],
        );
        overlap.cursor_minute = 9 * 60 + 15;
        assert!(overlap.start_edit(EditMode::Move));
        assert!(!overlap.apply_edit_step(15));
        let edit = overlap.edit.as_ref().unwrap();
        assert_eq!(edit.draft_start_minute, 9 * 60);
        assert_eq!(edit.draft_end_minute, 10 * 60);

        let mut invalid_duration =
            state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 9 * 60 + 15)]);
        invalid_duration.cursor_minute = 9 * 60 + 5;
        assert!(invalid_duration.start_edit(EditMode::Start));
        assert!(!invalid_duration.apply_edit_step(15));

        let mut bounds = state_with(0, 24 * 60, vec![view("edit", 0, 30)]);
        bounds.cursor_minute = 10;
        assert!(bounds.start_edit(EditMode::Move));
        assert!(!bounds.apply_edit_step(-15));
    }

    #[test]
    fn running_start_edit_blocks_now_day_bounds_and_overlaps() {
        let mut reaches_now = state_with(
            8 * 60,
            18 * 60,
            vec![running_view("running", 9 * 60 + 45, 10 * 60)],
        );
        set_now(&mut reaches_now, 10, 0);
        reaches_now.cursor_minute = 9 * 60 + 50;
        assert!(reaches_now.start_edit(EditMode::Start));
        assert!(!reaches_now.apply_edit_step(15));
        assert_eq!(
            reaches_now.edit.as_ref().unwrap().draft_start_minute,
            9 * 60 + 45
        );

        let mut bounds = state_with(0, 18 * 60, vec![running_view("running", 5, 60)]);
        set_now(&mut bounds, 1, 0);
        bounds.cursor_minute = 10;
        assert!(bounds.start_edit(EditMode::Start));
        assert!(!bounds.apply_edit_step(-15));
        assert_eq!(bounds.edit.as_ref().unwrap().draft_start_minute, 5);

        let mut overlap = state_with(
            0,
            18 * 60,
            vec![
                view("neighbor", 8 * 60 + 45, 9 * 60),
                running_view("running", 9 * 60, 10 * 60),
            ],
        );
        set_now(&mut overlap, 10, 0);
        overlap.cursor_minute = 9 * 60 + 10;
        assert!(overlap.start_edit(EditMode::Start));
        assert!(!overlap.apply_edit_step(-15));
        assert_eq!(overlap.edit.as_ref().unwrap().draft_start_minute, 9 * 60);
    }

    #[test]
    fn edit_cancel_restores_cursor_and_enter_without_changes_exits() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;
        assert!(state.start_edit(EditMode::End));
        assert_eq!(state.cursor_minute, 10 * 60);
        state.cancel_edit();
        assert!(state.edit.is_none());
        assert_eq!(state.cursor_minute, 9 * 60 + 15);

        assert!(state.start_edit(EditMode::Move));
        let client = ClockifyClient::new("secret".into(), NoopTransport);
        assert!(!execute_edit_commit(&client, "w1", &StoredConfig::default(), &mut state).unwrap());
        assert!(state.edit.is_none());
        assert_eq!(state.interaction.line(), Some("No changes."));
    }

    #[test]
    fn compute_blocks_uses_edit_draft_and_highlight() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;
        assert!(state.start_edit(EditMode::Move));
        assert!(state.apply_edit_step(15));

        let blocks = compute_blocks(&state, 0, 100);

        assert_eq!(blocks[0].start_col, col_at_minute(9 * 60 + 15, &state, 100));
        assert_eq!(blocks[0].highlight, BlockHighlight::EditMove);
    }

    #[test]
    fn running_start_edit_uses_dynamic_now_for_blocks_and_legend() {
        let mut state = state_with(
            8 * 60,
            18 * 60,
            vec![running_view("running", 9 * 60, 10 * 60)],
        );
        set_now(&mut state, 10, 0);
        state.cursor_minute = 9 * 60 + 10;
        assert!(state.start_edit(EditMode::Start));
        assert!(state.apply_edit_step(-15));

        let blocks = compute_blocks(&state, 0, 100);
        assert_eq!(blocks[0].start_col, col_at_minute(8 * 60 + 45, &state, 100));
        assert_eq!(blocks[0].highlight, BlockHighlight::EditStart);

        let legend = render_legend(&state, 120).join("\n");
        assert!(legend.contains("Time: 08:45–10:00 (running)"));
        assert!(legend.contains("Duration: 1h 15m"));

        set_now(&mut state, 10, 15);
        let legend = render_legend(&state, 120).join("\n");
        assert!(legend.contains("Time: 08:45–10:15 (running)"));
        assert!(legend.contains("Duration: 1h 30m"));
    }

    #[test]
    fn active_switch_start_update_changes_switched_start_only() {
        let config = StoredConfig {
            active_switch: Some(StoredSwitch {
                workspace_id: "w1".into(),
                user_id: "u1".into(),
                original_entry_id: "original".into(),
                switched_entry_id: "running".into(),
                switched_start: "2026-05-07T09:00:00Z".into(),
                return_start: Some("2026-05-07T08:00:00Z".into()),
                return_to: StoredTimerFields {
                    project_id: "p1".into(),
                    task_id: Some("t1".into()),
                    tag_ids: vec!["tag1".into()],
                    description: Some("Return".into()),
                },
            }),
            ..StoredConfig::default()
        };

        let updated =
            config_with_updated_active_switch_start(&config, "2026-05-07T08:45:00Z").unwrap();
        let active_switch = updated.active_switch.unwrap();

        assert_eq!(active_switch.switched_start, "2026-05-07T08:45:00Z");
        assert_eq!(active_switch.switched_entry_id, "running");
        assert_eq!(active_switch.return_to.project_id, "p1");
    }

    #[test]
    fn legend_uses_edit_draft_time_and_duration() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;
        assert!(state.start_edit(EditMode::Move));
        assert!(state.apply_edit_step(15));

        let legend = render_legend(&state, 120).join("\n");

        assert!(legend.contains("Time: 09:15–10:15"));
        assert!(legend.contains("Duration: 1h"));
    }

    #[test]
    fn edit_mode_accepts_only_edit_keys_and_ctrl_c() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("edit", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;
        assert!(state.start_edit(EditMode::Move));

        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Redraw
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::CommitEdit
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Beep
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::Redraw
        );
        assert!(state.edit.is_none());

        assert!(state.start_edit(EditMode::Move));
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &mut state,
                15,
                120,
            ),
            Action::Quit
        );
    }

    #[test]
    fn shortcut_set_is_single_source_for_render_and_dispatch() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("a", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 15;
        let shortcuts = build_shortcuts(&state, 120, 15);

        assert_eq!(shortcuts.top, global_shortcuts(&state, 120));
        assert_eq!(shortcuts.bottom, bottom_shortcuts(&state, 120, 15));
        assert_eq!(
            action_for_key(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
                &shortcuts.accepted,
            ),
            Some(Action::SplitCursorEntry)
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
                &mut state,
                15,
                120,
            ),
            Action::SplitCursorEntry
        );
    }

    #[test]
    fn hidden_shortcuts_beep_when_truncated() {
        let mut state = state_with(8 * 60, 18 * 60, vec![]);
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::Beep
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::Beep
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                &mut state,
                15,
                10
            ),
            Action::Beep
        );
    }

    #[test]
    fn confirmation_mode_accepts_only_confirm_cancel_and_ctrl_c() {
        let mut state = state_with(8 * 60, 18 * 60, vec![]);
        state.set_confirmation(
            "Delete entry e1? [Y/n]",
            TimelineAction::DeleteEntry {
                entry_id: "e1".into(),
            },
            "Delete cancelled.",
        );

        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::ConfirmYes
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::ConfirmYes
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
                &mut state,
                15,
                120
            ),
            Action::Beep
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &mut state,
                15,
                120
            ),
            Action::Quit
        );
    }

    #[test]
    fn cursor_timestamp_uses_selected_local_day_and_time() {
        let mut state = state_with(8 * 60, 18 * 60, vec![]);
        state.cursor_minute = 9 * 60 + 30;

        let timestamp = cursor_timestamp(&state).unwrap();
        let parsed = DateTime::parse_from_rfc3339(&timestamp)
            .unwrap()
            .with_timezone(&Local);

        assert_eq!(
            parsed.date_naive(),
            NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
        );
        assert_eq!(parsed.hour(), 9);
        assert_eq!(parsed.minute(), 30);
    }

    #[test]
    fn no_entry_action_sets_status_message() {
        let client = ClockifyClient::new("secret".into(), NoopTransport);
        let mut state = state_with(8 * 60, 18 * 60, vec![]);

        execute_timeline_action(
            &client,
            "w1",
            &StoredConfig::default(),
            &mut state,
            TimelineAction::StartTimerFromCursorEntry,
            false,
            false,
        )
        .unwrap();

        assert_eq!(state.interaction.line(), Some("No entry at cursor."));
    }

    #[test]
    fn start_action_is_blocked_but_split_running_entry_is_blocked_when_timer_is_running() {
        let client = ClockifyClient::new("secret".into(), NoopTransport);
        let mut running = view("running", 9 * 60, 10 * 60);
        running.running = true;
        let mut state = state_with(8 * 60, 18 * 60, vec![running]);
        state.current_timer_id = Some("running".into());
        state.cursor_minute = 9 * 60 + 10;

        execute_timeline_action(
            &client,
            "w1",
            &StoredConfig::default(),
            &mut state,
            TimelineAction::StartTimerFromCursorEntry,
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            state.interaction.line(),
            Some("Timer already running; stop it before starting another.")
        );

        execute_timeline_action(
            &client,
            "w1",
            &StoredConfig::default(),
            &mut state,
            TimelineAction::SplitCursorEntry,
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            state.interaction.line(),
            Some("Cannot split the running timer entry here.")
        );
    }

    #[test]
    fn delete_action_prompts_and_cancel_keeps_idle_message() {
        let mut state = state_with(8 * 60, 18 * 60, vec![view("e1", 9 * 60, 10 * 60)]);
        state.cursor_minute = 9 * 60 + 10;
        let client = ClockifyClient::new("secret".into(), NoopTransport);

        execute_timeline_action(
            &client,
            "w1",
            &StoredConfig::default(),
            &mut state,
            TimelineAction::DeleteCursorEntry,
            false,
            false,
        )
        .unwrap();

        assert_eq!(state.interaction.line(), Some("Delete entry e1? [Y/n]"));
        state.cancel_confirmation();
        assert_eq!(state.interaction.line(), Some("Delete cancelled."));
    }

    #[test]
    fn stop_action_prompts_before_api_call() {
        let mut state = state_with(8 * 60, 18 * 60, vec![]);

        state.set_confirmation(
            "Stop current timer? [Y/n]",
            TimelineAction::StopCurrentTimer,
            "Stop cancelled.",
        );

        assert_eq!(state.interaction.line(), Some("Stop current timer? [Y/n]"));
        state.cancel_confirmation();
        assert_eq!(state.interaction.line(), Some("Stop cancelled."));
    }

    #[test]
    fn merge_loaded_range_replaces_loading_days_in_order() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut days = loading_days(date - Duration::days(2), 3);
        merge_days(
            &mut days,
            vec![day_view(
                date - Duration::days(1),
                vec![view("loaded", 540, 600)],
            )],
        );
        assert_eq!(
            days.iter().map(|day| day.date).collect::<Vec<_>>(),
            vec![date - Duration::days(2), date - Duration::days(1), date]
        );
        assert_eq!(days[1].load_status, DayLoadStatus::Loaded);
        assert_eq!(days[1].entries[0].id, "loaded");
    }

    #[test]
    fn merging_older_range_preserves_selected_date() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(vec![
            day_view(date - Duration::days(1), vec![]),
            day_view(date, vec![]),
        ]);
        let selected_date = state.selected().date;
        merge_days(
            &mut state.days,
            vec![day_view(date - Duration::days(3), vec![])],
        );
        preserve_selected_date(&mut state, selected_date);
        assert_eq!(state.selected().date, selected_date);
        assert_eq!(state.days.first().unwrap().date, date - Duration::days(3));
    }

    #[test]
    fn repeated_request_does_not_enqueue_duplicate_in_flight_batch() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(loading_days(date - Duration::days(2), 3));
        let (loader, requests) = test_loader();
        request_oldest_loading_batch(&mut state, &loader);
        request_oldest_loading_batch(&mut state, &loader);
        assert!(state.loading.older_batch_in_flight);
        assert!(requests.try_recv().is_ok());
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn resizing_taller_prepends_loading_days_and_requests_batch() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(vec![
            day_view(date - Duration::days(1), vec![]),
            day_view(date, vec![]),
        ]);
        let (loader, requests) = test_loader();
        ensure_visible_days_requested(&mut state, 5, &loader);
        assert!(state.days.len() >= 5);
        assert_eq!(state.selected().date, date);
        let request = requests.try_recv().unwrap();
        match request {
            LoaderRequest::LoadRange {
                start, day_count, ..
            } => {
                assert_eq!(start, state.days.first().unwrap().date);
                assert_eq!(day_count, BATCH_DAY_COUNT);
            }
            LoaderRequest::Shutdown => panic!("unexpected shutdown request"),
        }
    }

    #[test]
    fn loading_older_days_selects_immediately_previous_day() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 8).unwrap();
        let mut state = multi_day_state(
            (0..5)
                .map(|i| day_view(date - Duration::days(4 - i as i64), vec![]))
                .collect(),
        );
        state.selected_day = 0;
        state.viewport_top = 0;
        let old_len = state.days.len();
        let old_oldest_date = state.days.first().unwrap().date;
        let (loader, requests) = test_loader();

        ensure_older_days_requested(&mut state, 5, &loader);

        assert_eq!(state.days.len(), old_len + BATCH_DAY_COUNT);
        assert_eq!(state.selected().date, old_oldest_date - Duration::days(1));
        assert_eq!(state.selected_day, BATCH_DAY_COUNT - 1);
        assert_eq!(state.viewport_top, 0);

        let request = requests.try_recv().unwrap();
        match request {
            LoaderRequest::LoadRange {
                start, day_count, ..
            } => {
                assert_eq!(start, state.days.first().unwrap().date);
                assert_eq!(day_count, BATCH_DAY_COUNT);
            }
            LoaderRequest::Shutdown => panic!("unexpected shutdown request"),
        }
    }

    #[test]
    fn failed_loader_event_marks_only_affected_days() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(loading_days(date - Duration::days(3), 4));
        let changed = apply_loader_event(
            &mut state,
            LoaderEvent::RangeFailed {
                generation: 0,
                start: date - Duration::days(2),
                day_count: 2,
                message: "network unavailable".into(),
            },
        );
        assert!(changed);
        assert_eq!(state.days[0].load_status, DayLoadStatus::Loading);
        assert_eq!(state.days[1].load_status, DayLoadStatus::Failed);
        assert_eq!(state.days[2].load_status, DayLoadStatus::Failed);
        assert_eq!(state.days[3].load_status, DayLoadStatus::Loading);
        assert!(state
            .loading
            .last_error
            .as_deref()
            .unwrap()
            .contains("network"));
    }

    #[test]
    fn stale_loader_event_is_ignored_after_reload_generation_changes() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(loading_days(date - Duration::days(1), 2));
        state.generation = 1;
        let changed = apply_loader_event(
            &mut state,
            LoaderEvent::RangeLoaded {
                generation: 0,
                start: date - Duration::days(1),
                days: vec![day_view(
                    date - Duration::days(1),
                    vec![view("old", 540, 600)],
                )],
            },
        );
        assert!(!changed);
        assert!(state.days[0].entries.is_empty());
    }

    #[test]
    fn header_status_reports_loading_and_errors() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(loading_days(date - Duration::days(1), 2));
        assert_eq!(header_status(&state), "loading older days...");
        state.days[0].load_status = DayLoadStatus::Failed;
        state.days[1].load_status = DayLoadStatus::Loaded;
        state.loading.last_error = Some("timeout".into());
        assert_eq!(header_status(&state), "load failed: timeout");
    }

    #[test]
    fn viewport_follows_selected_day_when_scrolling() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(
            (0..10)
                .map(|i| day_view(date - Duration::days(9 - i as i64), vec![]))
                .collect(),
        );
        state.viewport_top = 3;
        state.selected_day = 5;
        adjust_viewport(&mut state, 4);
        assert_eq!(state.viewport_top, 3);

        state.selected_day = 8;
        adjust_viewport(&mut state, 4);
        assert_eq!(
            state.viewport_top, 5,
            "viewport scrolls down with selection"
        );

        state.selected_day = 1;
        adjust_viewport(&mut state, 4);
        assert_eq!(state.viewport_top, 1, "viewport scrolls up with selection");
    }

    #[test]
    fn up_down_keys_change_selected_day_within_bounds() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
        let mut state = multi_day_state(vec![
            day_view(date - Duration::days(2), vec![]),
            day_view(date - Duration::days(1), vec![]),
            day_view(date, vec![]),
        ]);
        assert_eq!(state.selected_day, 2);
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        handle_key(up, &mut state, 15, 120);
        assert_eq!(state.selected_day, 1);
        handle_key(up, &mut state, 15, 120);
        handle_key(up, &mut state, 15, 120);
        assert_eq!(state.selected_day, 0, "should clamp at 0");
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        handle_key(down, &mut state, 15, 120);
        assert_eq!(state.selected_day, 1);
    }
}
