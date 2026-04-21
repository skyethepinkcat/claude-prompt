use colored::ColoredString;
use colored::Colorize;
use git2::Repository;
use serde::Deserialize;
use std::io;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Model {
    id: String,
    display_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Workspace {
    current_dir: String,
    project_dir: String,
    added_dirs: Vec<String>,
    git_worktree: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct OutputStyle {
    name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Cost {
    total_cost_usd: f64,
    total_duration_ms: u64,
    total_api_duration_ms: u64,
    total_lines_added: u64,
    total_lines_removed: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct CurrentUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ContextWindow {
    total_input_tokens: u64,
    total_output_tokens: u64,
    context_window_size: u64,
    used_percentage: u64,
    remaining_percentage: u64,
    current_usage: CurrentUsage,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct RateLimitWindow {
    used_percentage: f64,
    resets_at: i64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct RateLimits {
    five_hour: RateLimitWindow,
    seven_day: RateLimitWindow,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Vim {
    mode: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Agent {
    name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Worktree {
    name: String,
    path: String,
    branch: String,
    original_cwd: String,
    original_branch: String,
}
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ClaudeContext {
    cwd: String,
    session_id: String,
    session_name: String,
    transcript_path: String,
    model: Model,
    workspace: Workspace,
    version: String,
    output_style: OutputStyle,
    cost: Cost,
    context_window: ContextWindow,
    exceeds_200k_tokens: bool,
    rate_limits: RateLimits,
    vim: Option<Vim>,
    agent: Option<Agent>,
    worktree: Option<Worktree>,
}

fn path_section(ctx: &ClaudeContext) -> Option<ColoredString> {
    let path = ctx.cwd.split("/").last()?;

    Some(path.clear())
}

fn git_section(_ctx: &ClaudeContext) -> Option<ColoredString> {
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(_) => return None,
    };

    let branch = match repo.head() {
        Ok(b) => b,
        Err(_) => return None,
    };

    let out = format!("on  {}", branch.shorthand()?);

    Some(out.normal())
}

fn model_section(ctx: &ClaudeContext) -> Option<ColoredString> {
    let model = &ctx.model.display_name;

    let out: ColoredString;
    if model.to_lowercase() == "opus" {
        out = format!("[ {}]", model).red();
    } else if model.to_lowercase() == "haiku" {
        out = format!("[ {}]", model).yellow();
    } else {
        return None;
    }

    Some(out)
}

fn caveman_section(_: &ClaudeContext) -> Option<ColoredString> {
    use std::path::PathBuf;

    let home = std::env::home_dir()?.display().to_string();

    let flagfile: PathBuf = [&home, ".claude", ".caveman-active"].iter().collect();

    if !flagfile.exists() {
        return Some("[ NO CAVEMAN]".yellow());
    }

    let mode = std::fs::read_to_string(flagfile).unwrap();

    let modestring = if mode == "full" || mode.is_empty() {
        String::from("")
    } else {
        format!(":{}", mode)
    };

    Some(format!("[CAVEMAN{}]", modestring).normal())
}

fn session_section(ctx: &ClaudeContext) -> Option<ColoredString> {
    use chrono::{DateTime, Local};
    let five_hour = &ctx.rate_limits.five_hour;
    let resets_at: DateTime<Local> = DateTime::from_timestamp(five_hour.resets_at, 0)?.into();

    let mut out_str = format!("{}% usage", five_hour.used_percentage);

    if five_hour.used_percentage > 25.0 {
        out_str = format!("{} until {}", out_str, resets_at.format("%H:%M"));
    }

    out_str = format!("[{}]", out_str);

    let out = if five_hour.used_percentage >= 75.0 {
        out_str.bright_red()
    } else if five_hour.used_percentage >= 50.0 {
        out_str.yellow()
    } else {
        out_str.normal()
    };

    Some(out)
}

fn weekly_session(ctx: &ClaudeContext) -> Option<ColoredString> {
    use chrono::{DateTime, Datelike, Local, Timelike};
    let now: DateTime<Local> = Local::now();
    let weekly = &ctx.rate_limits.seven_day;
    let resets_at: DateTime<Local> = DateTime::from_timestamp(weekly.resets_at, 0)?.into();
    let week_percentage: f64 = ((((now.weekday().num_days_from_sunday() * 86400)
        + (now.hour() * 3600)
        + (now.minute() * 60))
        * 100) // Multiply by 100 here to get an integer result and never touch floating points.
        / (7 * 86400))
        .into();

    let out = if week_percentage < weekly.used_percentage {
        format!(
            " {}% weekly usage until {}",
            weekly.used_percentage,
            resets_at.format("%b %-d %H %M")
        )
        .bright_red()
    } else {
        "".normal()
    };

    Some(out)
}

fn push_if_valid(array: &mut Vec<String>, input: Option<ColoredString>) {
    let string = match input {
        Some(s) => s,
        None => return,
    };

    array.push(string.to_string());
}

fn get_prompt() -> io::Result<Vec<String>> {
    let stdin = io::stdin();
    let ctx: ClaudeContext = serde_json::from_reader(stdin)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut sections: Vec<Vec<String>> = Vec::new();
    sections.push(Vec::new());
    sections.push(Vec::new());

    push_if_valid(&mut sections[0], weekly_session(&ctx));
    push_if_valid(&mut sections[1], path_section(&ctx));
    push_if_valid(&mut sections[1], git_section(&ctx));
    push_if_valid(&mut sections[1], model_section(&ctx));
    push_if_valid(&mut sections[1], caveman_section(&ctx));
    push_if_valid(&mut sections[1], session_section(&ctx));

    Ok(sections.iter().filter(|s| !s.is_empty()).map(|s| s.join(" ")).collect())
}

fn main() -> io::Result<()> {
    let prompts = match get_prompt() {
        Ok(s) => s,
        Err(e) => Vec::from([String::from("Loading prompt..."), e.to_string()]),
    };


    for p in prompts {
        println!("{}", p);
    }


    Ok(())
}
