use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Europe::Paris;

use crate::state::PsMatch;

pub fn format_announced(match_data: &PsMatch) -> String {
    let (opponent_name, tournament_name, formatted_time, stream_url) =
        extract_match_details(match_data);

    format!(
        r#"🎮 𝗠𝗔𝗧𝗖𝗛 𝗔𝗡𝗡𝗢𝗡𝗖𝗘́

<b>{opponent_name}</b>
🏆 {tournament_name}
📅 {formatted_time}
🎯 BO{}

📺 Stream FR : {}"#,
        match_data
            .number_of_games
            .map_or("3".to_string(), |n| n.to_string()),
        stream_url.unwrap_or_else(|| "Aucun stream disponible".to_string()),
    )
}

pub fn format_live(match_data: &PsMatch) -> String {
    let (opponent_name, tournament_name, _, stream_url) = extract_match_details(match_data);

    format!(
        r#"🔴 𝗟𝗜𝗩𝗘 𝗠𝗔𝗜𝗡𝗧𝗘𝗡𝗔𝗡𝗧

<b>{opponent_name}</b>
🏆 {tournament_name} · BO{}

📺 {}"#,
        match_data
            .number_of_games
            .map_or("3".to_string(), |n| n.to_string()),
        stream_url.unwrap_or_else(|| "Aucun stream disponible".to_string()),
    )
}

pub fn format_result(match_data: &PsMatch, vitality_team_id: i64) -> String {
    let (opponent_name, tournament_name, formatted_time, _) = extract_match_details(match_data);
    let (vitality_score, opponent_score) = get_scores(match_data, vitality_team_id);

    if vitality_score > opponent_score {
        format!(
            r#"🏆 𝗩𝗜𝗖𝗧𝗢𝗜𝗥𝗘 !

<b>{vitality_score} - {opponent_score}</b> {opponent_name}
🏆 {tournament_name} · BO{}
📅 {formatted_time}"#,
            match_data
                .number_of_games
                .map_or("3".to_string(), |n| n.to_string()),
        )
    } else {
        format!(
            r#"💀 𝗗𝗘́𝗙𝗔𝗜𝗧𝗘

<b>{vitality_score} - {opponent_score}</b> {opponent_name}
🏆 {tournament_name} · BO{}
📅 {formatted_time}"#,
            match_data
                .number_of_games
                .map_or("3".to_string(), |n| n.to_string()),
        )
    }
}

pub fn format_daily_reminder(matches: &[&PsMatch]) -> String {
    if matches.is_empty() {
        return "☀️ 𝗠𝗔𝗧𝗖𝗛 𝗔𝗨𝗝𝗢𝗨𝗥𝗗'𝗛𝗨𝗜\n\nAucun match prévu.".to_string();
    }

    let mut msg = "☀️ 𝗠𝗔𝗧𝗖𝗛 𝗔𝗨𝗝𝗢𝗨𝗥𝗗'𝗛𝗨𝗜\n\n".to_string();
    msg.push_str("Vitality joue aujourd'hui !\n\n");

    for match_data in matches {
        let (opponent_name, tournament_name, formatted_time, stream_url) =
            extract_match_details(match_data);
        msg.push_str(&format!(
            "⏰ {} — vs {} (BO{})\n   🏆 {}\n   📺 {}\n\n",
            formatted_time,
            opponent_name,
            match_data
                .number_of_games
                .map_or("3".to_string(), |n| n.to_string()),
            tournament_name,
            stream_url.unwrap_or_else(|| "Aucun stream disponible".to_string()),
        ));
    }

    msg.trim_end().to_string()
}

pub fn format_result_24h(match_data: &PsMatch, vitality_team_id: i64) -> String {
    let (opponent_name, tournament_name, formatted_time, _) = extract_match_details(match_data);
    let (vitality_score, opponent_score) = get_scores(match_data, vitality_team_id);

    if vitality_score > opponent_score {
        format!(
            r#"🏆 𝗩𝗜𝗖𝗧𝗢𝗜𝗥𝗘 ! (24h après la fin)

<b>{vitality_score} - {opponent_score}</b> {opponent_name}
🏆 {tournament_name} · BO{}
📅 {formatted_time}"#,
            match_data
                .number_of_games
                .map_or("3".to_string(), |n| n.to_string()),
        )
    } else {
        format!(
            r#"💀 𝗗𝗘́𝗙𝗔𝗜𝗧𝗘 (24h après la fin)

<b>{vitality_score} - {opponent_score}</b> {opponent_name}
🏆 {tournament_name} · BO{}
📅 {formatted_time}"#,
            match_data
                .number_of_games
                .map_or("3".to_string(), |n| n.to_string()),
        )
    }
}

fn extract_match_details(match_data: &PsMatch) -> (String, String, String, Option<String>) {
    let opponent_name = if let Some(wrapper) = match_data.opponents.first() {
        if wrapper.opponent.id == 9565 {
            // Vitality team ID
            match_data
                .opponents
                .get(1)
                .map_or("Unknown".to_string(), |w| w.opponent.name.clone())
        } else {
            wrapper.opponent.name.clone()
        }
    } else {
        "Unknown".to_string()
    };

    let tournament_name = match_data
        .tournament
        .as_ref()
        .map_or("Tournoi inconnu".to_string(), |t| t.name.clone());

    // Format time in French
    let formatted_time = match_data
        .begin_at
        .as_ref()
        .map_or("Heure inconnue".to_string(), |time| format_match_time(time));

    // Find stream URL
    let stream_url = match_data.streams_list.as_ref().and_then(|streams| {
        streams
            .iter()
            .find(|s| s.language.as_ref().map_or(false, |l| l == "fr"))
            .or_else(|| {
                streams
                    .iter()
                    .find(|s| s.language.as_ref().map_or(false, |l| l == "en"))
            })
            .and_then(|s| s.raw_url.clone())
    });

    (opponent_name, tournament_name, formatted_time, stream_url)
}

fn get_scores(match_data: &PsMatch, vitality_team_id: i64) -> (i32, i32) {
    let mut vitality_score = 0;
    let mut opponent_score = 0;

    if let Some(results) = &match_data.results {
        for result in results {
            if result.team_id == vitality_team_id {
                vitality_score = result.score;
            } else {
                opponent_score = result.score;
            }
        }
    }

    (vitality_score, opponent_score)
}

fn format_match_time(utc_str: &str) -> String {
    let utc = DateTime::parse_from_rfc3339(utc_str)
        .unwrap_or_else(|_| Utc::now().into())
        .with_timezone(&Utc);
    let local = utc.with_timezone(&Paris);

    let day = match local.weekday() {
        chrono::Weekday::Mon => "Lundi",
        chrono::Weekday::Tue => "Mardi",
        chrono::Weekday::Wed => "Mercredi",
        chrono::Weekday::Thu => "Jeudi",
        chrono::Weekday::Fri => "Vendredi",
        chrono::Weekday::Sat => "Samedi",
        chrono::Weekday::Sun => "Dimanche",
    };

    let month = match local.month() {
        1 => "janvier",
        2 => "février",
        3 => "mars",
        4 => "avril",
        5 => "mai",
        6 => "juin",
        7 => "juillet",
        8 => "août",
        9 => "septembre",
        10 => "octobre",
        11 => "novembre",
        12 => "décembre",
        _ => "",
    };

    format!(
        "{} {} {} · {}h{:02}",
        day,
        local.day(),
        month,
        local.hour(),
        local.minute()
    )
}

pub fn match_is_today(match_data: &PsMatch) -> bool {
    if let Some(begin_at) = &match_data.begin_at {
        if let Ok(utc) = DateTime::parse_from_rfc3339(begin_at) {
            let local = utc.with_timezone(&Paris);
            let today = Utc::now().with_timezone(&Paris).date_naive();
            return local.date_naive() == today;
        }
    }
    false
}
