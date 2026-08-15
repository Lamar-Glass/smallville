use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::agent::{Agent, PlanEntry};
use crate::config::Config;
use crate::llm::LlmClient;
use crate::memory::MemoryKind;
use crate::state::{Event, SharedState};
use crate::town;

const PLAN_HOURS: std::ops::RangeInclusive<u8> = 8..=22;
const MAX_DIALOGUE_GROUPS_PER_HOUR: usize = 3;

pub fn run(config: &Config, shared: &Arc<RwLock<SharedState>>) -> Result<(), String> {
    let client = Arc::new(LlmClient::from_env(&config.model, &config.embed_model, &config.base_url)?);
    let mut agents = build_agents();

    println!(
        "Smallville simulation started: {} residents, {} locations, {} real-seconds per in-game hour",
        agents.len(),
        town::LOCATIONS.len(),
        config.hour_seconds
    );
    if client.mock {
        println!("SIM_MOCK=1: running with scripted responses (no API calls)");
    } else {
        println!("model: {}  embed: {}", config.model, config.embed_model);
    }
    println!("Open http://localhost:{} in your browser to watch.", config.port);

    let mut day: u32 = 1;
    let mut day_events: Vec<String> = Vec::new();

    loop {
        for hour in PLAN_HOURS {
            thread::sleep(Duration::from_secs(config.hour_seconds));

            let mut events: Vec<Event> = Vec::new();

            if hour == 8 {
                generate_plans(&client, &mut agents, day, shared)?;
                reset_daily_flags(&mut agents);
            }

            events.extend(execute_hour(&mut agents, day, hour));
            events.extend(run_conversations(&client, &mut agents, day, hour));
            events.extend(reflect_if_due(&client, &mut agents, day, hour)?);

            if hour == 22 {
                let summary = daily_summary(&client, &agents, day, &day_events)?;
                day_events.clear();
                events.push(summary);
                for a in agents.iter_mut() {
                    if a.location != a.home {
                        events.push(move_event(a, a.home, "going home to sleep"));
                    }
                    a.location = a.home;
                    a.current_action = "sleeping".to_string();
                }
            }

            for e in &events {
                day_events.push(e.text.clone());
            }
            for e in events {
                shared.write().unwrap().push(e);
            }
            update_snapshot(shared, &agents, day, hour);
        }
        day += 1;
    }
}

fn build_agents() -> Vec<Agent> {
    let home_lin = 0;
    let home_cafe = 1;
    let home_market = 2;
    let home_park = 3;
    let home_library = 4;
    let home_hall = 5;

    vec![
        Agent::new(0, "John Lin", "🧔", "#e8a33d", "A pharmacy shopkeeper who loves helping his neighbors and tinkering in his workshop.", "kind, curious, slightly forgetful, a family man", home_lin, 2),
        Agent::new(1, "Eddy Lin", "🧑", "#7fb069", "John's son, a college student studying music theory, shy but warm once he opens up.", "quiet, musical, idealistic", home_lin, 1),
        Agent::new(2, "Sam Moore", "🧑‍🌾", "#6bbf59", "A gardener who tends the town park and can talk about flowers for hours.", "patient, cheerful, observant", home_park, 3),
        Agent::new(3, "Latoya Williams", "👩🏾", "#d47f4f", "A botanist who spends her days cataloging plants at the library.", "calm, detail-oriented, loves family", home_library, 4),
        Agent::new(4, "Isabella Rodriguez", "👩🏽", "#c25b5b", "The owner of Hobbit Hole Café, the social heart of town.", "sociable, generous, a sharp memory for gossip", home_cafe, 1),
        Agent::new(5, "Tom Moreno", "👨", "#5b8dc2", "Co-owner of Willow Market, always ready for a chat at the register.", "friendly, talkative, competitive", home_market, 2),
        Agent::new(6, "Jane Moreno", "👩🏻", "#9a6fb0", "Co-owner of Willow Market, a little more reserved than her husband.", "conservative, dependable, values tradition", home_market, 2),
        Agent::new(7, "Carlos Gomez", "👨🏽", "#4fb0a8", "A lawyer who keeps to himself but notices everything.", "quiet, ambitious, observant", home_hall, 5),
        Agent::new(8, "Wolfgang Schulz", "👨‍🎓", "#c98a2b", "A college student passionate about activism, hiking, and big ideas.", "idealistic, energetic, principled", home_park, 3),
        Agent::new(9, "Hailey Johnson", "👩🏼", "#c2628e", "A pharmacy student who works part-time at the café and dreams of opening her own shop.", "cheerful, ambitious, a little restless", home_cafe, 1),
    ]
}

fn reset_daily_flags(agents: &mut [Agent]) {
    for a in agents.iter_mut() {
        a.reflections_today = 0;
    }
}

fn time_string(hour: u8) -> String {
    format!("{:02}:00", hour)
}

fn update_snapshot(shared: &Arc<RwLock<SharedState>>, agents: &[Agent], day: u32, hour: u8) {
    shared.write().unwrap().set_agents(day, hour, agents);
}

fn move_event(a: &Agent, to: usize, action: &str) -> Event {
    Event {
        id: 0,
        day: 0,
        time: String::new(),
        kind: "move".to_string(),
        text: format!(
            "{} {} moved to {} — {}",
            town::emoji(to),
            a.name,
            town::name(to),
            action
        ),
    }
}

fn generate_plans(
    client: &Arc<LlmClient>,
    agents: &mut [Agent],
    day: u32,
    shared: &Arc<RwLock<SharedState>>,
) -> Result<(), String> {
    thread::scope(|s| {
        for agent in agents.iter_mut() {
            s.spawn(|| {
                match generate_plan(client, agent, day) {
                    Ok(plan) => agent.plan = plan,
                    Err(e) => {
                        eprintln!("warning: plan generation failed for {}: {e}", agent.name);
                        agent.plan = default_plan(agent);
                    }
                }
                agent.last_plan_day = day;
            });
        }
    });
    shared.write().unwrap().push(Event {
        id: 0,
        day,
        time: time_string(8),
        kind: "system".to_string(),
        text: format!(
            "☀️ Day {day} — {} residents formed their morning plans.",
            agents.len()
        ),
    });
    Ok(())
}

fn generate_plan(client: &Arc<LlmClient>, agent: &Agent, _day: u32) -> Result<Vec<PlanEntry>, String> {
    let locations: Vec<&str> = town::LOCATIONS.iter().map(|l| l.name).collect();
    let system = "You are the mind of an autonomous agent living in the small town of Smallville. \
        You generate your own daily schedule.";
    let user = format!(
        "Character: {}.\nTraits: {}.\n\nYour home is {} and you usually work around {}.\n\
        Produce today's schedule as exactly 15 lines, one per hour from 08:00 to 22:00, in this format:\n\
        HH:00|LOCATION NAME|short action description\n\
        Use only these location names: {}",
        agent.bio,
        agent.traits,
        town::name(agent.home),
        town::name(agent.work),
        locations.join(", ")
    );
    let text = client.chat(system, &user)?;
    let plan = parse_plan(&text, agent);
    if plan.is_empty() {
        Ok(default_plan(agent))
    } else {
        Ok(plan)
    }
}

fn parse_plan(text: &str, agent: &Agent) -> Vec<PlanEntry> {
    let mut plan = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 3 {
            continue;
        }
        let Some(hour) = parse_hour(parts[0]) else { continue };
        let location = town::find(parts[1]).unwrap_or(agent.home);
        let action = parts[2].trim();
        if action.is_empty() {
            continue;
        }
        plan.push(PlanEntry {
            hour,
            location,
            action: action.to_string(),
        });
    }
    plan
}

fn parse_hour(s: &str) -> Option<u8> {
    let h = s.trim().split(':').next()?.parse::<u8>().ok()?;
    if PLAN_HOURS.contains(&h) {
        Some(h)
    } else {
        None
    }
}

fn default_plan(agent: &Agent) -> Vec<PlanEntry> {
    let mut plan = Vec::new();
    let steps: Vec<(u8, usize, &str)> = vec![
        (8, agent.home, "having breakfast and reading the paper"),
        (9, agent.work, "opening up and getting settled"),
        (10, agent.work, "working and helping neighbors"),
        (11, agent.work, "working"),
        (12, 1, "grabbing lunch at the café"),
        (13, agent.work, "working through the afternoon"),
        (15, 3, "taking a walk in the park"),
        (16, 3, "sitting in the park and thinking"),
        (17, agent.home, "running errands and preparing dinner"),
        (18, agent.home, "making dinner"),
        (19, agent.home, "eating dinner"),
        (20, agent.home, "relaxing and reading"),
        (21, agent.home, "unwinding before bed"),
        (22, agent.home, "getting ready to sleep"),
    ];
    for (hour, location, action) in steps {
        plan.push(PlanEntry {
            hour,
            location,
            action: action.to_string(),
        });
    }
    plan
}

fn execute_hour(agents: &mut [Agent], day: u32, hour: u8) -> Vec<Event> {
    let mut events = Vec::new();
    for a in agents.iter_mut() {
        let next = a
            .plan
            .iter()
            .find(|p| p.hour == hour)
            .map(|p| (p.location, p.action.clone()))
            .unwrap_or_else(|| (a.home, "resting".to_string()));

        let now = minutes(day, hour);
        a.memory.add(
            &format!(
                "At {:02}:00 I was at {} and I was {}.",
                hour,
                town::name(next.0),
                next.1
            ),
            importance_heuristic(&next.1),
            now,
            MemoryKind::Observation,
            None,
        );

        if a.location != next.0 {
            events.push(move_event(a, next.0, &next.1));
            a.location = next.0;
        } else if a.current_action != next.1 {
            events.push(Event {
                id: 0,
                day,
                time: time_string(hour),
                kind: "act".to_string(),
                text: format!("{} is {}", a.name, next.1),
            });
        }
        a.current_action = next.1;
    }
    events
}

fn minutes(day: u32, hour: u8) -> f64 {
    (day as f64) * 1440.0 + (hour as f64) * 60.0
}

fn run_conversations(
    client: &Arc<LlmClient>,
    agents: &mut [Agent],
    day: u32,
    hour: u8,
) -> Vec<Event> {
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, a) in agents.iter().enumerate() {
        groups.entry(a.location).or_default().push(i);
    }

    let mut events = Vec::new();
    let mut groups_done = 0usize;

    for (loc, members) in groups {
        if members.len() < 2 || groups_done >= MAX_DIALOGUE_GROUPS_PER_HOUR {
            continue;
        }
        let participants: Vec<usize> = members[..members.len().min(3)].to_vec();
        let names: Vec<String> = participants
            .iter()
            .map(|&i| agents[i].name.clone())
            .collect();
        let now = minutes(day, hour);

        let query = format!(
            "talking with {} at {}",
            names.join(", "),
            town::name(loc)
        );
        let query_embed = client.embed(&query).ok();

        let context: Vec<String> = participants
            .iter()
            .map(|&i| {
                let a = &agents[i];
                let mems = a
                    .memory
                    .retrieve(&query, query_embed.as_ref(), now, 3);
                format!(
                    "{} — {}. Traits: {}. Recalled memories: {}",
                    a.name,
                    a.bio,
                    a.traits,
                    if mems.is_empty() { "none".to_string() } else { mems.join(" | ") }
                )
            })
            .collect();

        let system = "You are a conversation among residents of Smallville. \
            Keep it short and natural, 3-4 lines total, one per speaker. \
            Each line MUST start with the speaker's exact name followed by a colon.";
        let user = format!(
            "Dialogue between: {}\nLocation: {}\n\nParticipants:\n{}\n\nGenerate the dialogue now.",
            names.join(", "),
            town::name(loc),
            context.join("\n")
        );

        let Ok(text) = client.chat(system, &user) else {
            continue;
        };

        let lines = parse_dialogue(&text, &names);
        if lines.is_empty() {
            continue;
        }

        groups_done += 1;

        for &i in &participants {
            for &j in &participants {
                if i != j {
                    let other_name = agents[j].name.clone();
                    let entry = agents[i].relationships.entry(other_name).or_insert(0);
                    *entry += 1;
                }
            }
        }

        for line in &lines {
            for &i in &participants {
                if line.starts_with(&agents[i].name) {
                    agents[i].memory.add(
                        &format!("At {}: {line}", town::name(loc)),
                        importance_heuristic(line),
                        now,
                        MemoryKind::Dialogue,
                        client.embed(line).ok(),
                    );
                }
            }
            events.push(Event {
                id: 0,
                day,
                time: time_string(hour),
                kind: "dialogue".to_string(),
                text: line.clone(),
            });
        }
    }
    events
}

fn parse_dialogue(text: &str, names: &[String]) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() {
                return None;
            }
            if names.iter().any(|n| l.starts_with(n.as_str())) {
                Some(l.to_string())
            } else {
                None
            }
        })
        .take(6)
        .collect()
}

fn reflect_if_due(
    client: &Arc<LlmClient>,
    agents: &mut [Agent],
    day: u32,
    hour: u8,
) -> Result<Vec<Event>, String> {
    if !hour.is_multiple_of(4) {
        return Ok(Vec::new());
    }
    let now = minutes(day, hour);
    let mut events = Vec::new();

    for a in agents.iter_mut() {
        if a.memory.len() < 8 || a.reflections_today >= 3 {
            continue;
        }
        let recent = a.memory.recent(12);
        if recent.is_empty() {
            continue;
        }
        let system = "You are an autonomous agent reflecting on your day in Smallville. \
            Produce exactly 3 concise insights about yourself or the town, one per line.";
        let user = format!(
            "Your recent experiences:\n{}\n\nReflections:",
            recent.join("\n")
        );
        let Ok(text) = client.chat(system, &user) else {
            continue;
        };
        let insights: Vec<String> = text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .take(3)
            .collect();
        if insights.is_empty() {
            continue;
        }
        a.reflections_today += 1;
        for insight in insights {
            a.memory.add(
                &insight,
                importance_heuristic(&insight) + 1.0,
                now,
                MemoryKind::Reflection,
                client.embed(&insight).ok(),
            );
            events.push(Event {
                id: 0,
                day,
                time: time_string(hour),
                kind: "reflection".to_string(),
                text: format!("🧠 {} reflected: {insight}", a.name),
            });
        }
    }
    Ok(events)
}

fn daily_summary(
    client: &Arc<LlmClient>,
    agents: &[Agent],
    day: u32,
    day_events: &[String],
) -> Result<Event, String> {
    let mut sample: Vec<String> = day_events.iter().rev().take(40).cloned().collect();
    sample.reverse();
    let system = "You are the town chronicler of Smallville. Summarize the day in 2-3 sentences, \
        noting anything unusual, meaningful conversations, or how the town is coming together.";
    let user = format!(
        "Today's events:\n{}\n\nDay {day} summary:",
        sample.join("\n")
    );
    let text = client.chat(system, &user)?;
    Ok(Event {
        id: 0,
        day,
        time: time_string(22),
        kind: "summary".to_string(),
        text: format!(
            "📜 Day {day} came to a close. {} residents turned in.\n    {text}",
            agents.len()
        ),
    })
}

fn importance_heuristic(text: &str) -> f32 {
    let lower = text.to_lowercase();
    let mut score = 2.0f32;
    for kw in [
        "friend", "family", "love", "hate", "fight", "party", "secret", "help", "money",
        "dream", "afraid", "miss", "invite", "plan", "idea", "learn", "argument",
    ] {
        if lower.contains(kw) {
            score += 1.0;
        }
    }
    score.min(10.0)
}
