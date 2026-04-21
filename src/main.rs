use colored::ColoredString;
use colored::Colorize;
use git2::Repository;
use serde_json::Value;
use std::io;
use to_be;
use to_be::Truthy;

fn result_is_truthy<T: Truthy, E>(inpt: Result<T, E>) -> bool {
    match inpt {
        Ok(x) => x.is_truey(),
        Err(_) => false,
    }
}

fn is_verbose() -> bool {
    result_is_truthy(std::env::var("CLAUDE_VERBOSE_STATUS"))
}

fn path_section(ctx: &Value) -> Option<ColoredString> {
    let cwd = ctx["cwd"].as_str()?;
    let path = cwd.split("/").last()?;

    Some(path.clear())
}

fn git_section(_ctx: &Value) -> Option<ColoredString> {
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

fn model_section(ctx: &Value) -> Option<ColoredString> {
    let model = ctx["model"]["display_name"].as_str()?;

    let out: ColoredString;
    if model.to_lowercase() == "opus" {
        out = format!("[ {}]", model).red();
    } else if model.to_lowercase() == "haiku" {
        out = format!("[ {}]", model).yellow();
    } else if is_verbose() {
        out = format!("[{}]", model).normal();
    } else {
        return None;
    }

    Some(out)
}

fn caveman_section(_: &Value) -> Option<ColoredString> {
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

fn session_section(ctx: &Value) -> Option<ColoredString> {
    use chrono::{DateTime, Local};
    let five_hour = &ctx["rate_limits"]["five_hour"];
    let resets_at_int = match five_hour["resets_at"].as_i64() {
        Some(i) => i,
        None => return Some("[Waiting for API...]".yellow()),
    };
    let resets_at: DateTime<Local> = DateTime::from_timestamp(resets_at_int, 0)?.into();
    let used_percentage = five_hour["used_percentage"].as_f64()?;

    let mut out_str = format!("{}% usage", used_percentage);

    if used_percentage > 25.0 {
        out_str = format!("{} until {}", out_str, resets_at.format("%H:%M"));
    }

    out_str = format!("[{}]", out_str);

    Some(if used_percentage >= 75.0 {
        out_str.bright_red()
    } else if used_percentage >= 50.0 {
        out_str.yellow()
    } else {
        out_str.normal()
    })
}

fn weekly_section(ctx: &Value) -> Option<ColoredString> {
    use chrono::{DateTime, Datelike, Local, Timelike};
    let now: DateTime<Local> = Local::now();
    let weekly = &ctx["rate_limits"]["seven_day"];
    let resets_at: DateTime<Local> =
        DateTime::from_timestamp(weekly["resets_at"].as_i64()?, 0)?.into();
    let week_percentage: f64 = ((((now.weekday().num_days_from_sunday() * 86400)
        + (now.hour() * 3600)
        + (now.minute() * 60))
        * 100) // Multiply by 100 here to get an integer result and never touch floating points.
        / (7 * 86400))
        .into();

    let used_percentage = weekly["used_percentage"].as_f64()?;
    let out = if week_percentage < used_percentage {
        format!(
            " {}% weekly usage until {}",
            used_percentage,
            resets_at.format("%b %-d")
        )
        .bright_red()
    } else {
        return None;
    };

    Some(out)
}

fn context_section(ctx: &Value) -> Option<ColoredString> {
    let used_percentage = ctx["context_window"]["used_percentage"].as_u64()?;

    if used_percentage > 75 {
        Some(format!("[ {}% context]", used_percentage).bright_red())
    } else if used_percentage > 50 {
        Some(format!("[ {}% context]", used_percentage).yellow())
    } else if used_percentage > 25 || is_verbose() {
        Some(format!("[{}% context]", used_percentage).normal())
    } else {
        None
    }
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
    let ctx: serde_json::Value = serde_json::from_reader(stdin)?;

    let mut sections: Vec<Vec<String>> = vec![vec![], vec![]];

    push_if_valid(&mut sections[0], weekly_section(&ctx));
    push_if_valid(&mut sections[1], path_section(&ctx));
    push_if_valid(&mut sections[1], git_section(&ctx));
    push_if_valid(&mut sections[1], model_section(&ctx));
    push_if_valid(&mut sections[1], caveman_section(&ctx));
    push_if_valid(&mut sections[1], session_section(&ctx));
    push_if_valid(&mut sections[1], context_section(&ctx));

    Ok(sections
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.join(" "))
        .collect())
}

fn main() -> io::Result<()> {
    // colored incorrectly assumes that claude can't handle colored strings, so we need to force it.
    colored::control::set_override(true);

    let prompts = match get_prompt() {
        Ok(s) => s,
        Err(e) => vec![String::from("Prompt Error"), e.to_string()],
    };

    for p in prompts {
        println!("{}", p);
    }

    Ok(())
}
