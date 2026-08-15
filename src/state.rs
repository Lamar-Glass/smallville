use std::collections::VecDeque;

use serde::Serialize;

use crate::agent::Agent;
use crate::town;

const MAX_EVENTS: usize = 500;

#[derive(Clone, Serialize)]
pub struct AgentView {
    pub id: usize,
    pub name: String,
    pub emoji: String,
    pub color: &'static str,
    pub bio: String,
    pub traits: String,
    pub location: usize,
    pub location_name: String,
    pub location_emoji: String,
    pub current_action: String,
    pub plan: Vec<String>,
    pub recent_memories: Vec<String>,
    pub relationships: Vec<(String, i32)>,
}

impl From<&Agent> for AgentView {
    fn from(a: &Agent) -> Self {
        let mut plan: Vec<String> = a
            .plan
            .iter()
            .map(|p| format!("{:02}:00 — {}", p.hour, p.action))
            .collect();
        plan.sort();
        let mut relationships: Vec<(String, i32)> =
            a.relationships.iter().map(|(k, v)| (k.clone(), *v)).collect();
        relationships.sort_by_key(|x| std::cmp::Reverse(x.1));
        relationships.truncate(3);
        Self {
            id: a.id,
            name: a.name.clone(),
            emoji: a.emoji.to_string(),
            color: a.color,
            bio: a.bio.clone(),
            traits: a.traits.clone(),
            location: a.location,
            location_name: town::name(a.location).to_string(),
            location_emoji: town::emoji(a.location).to_string(),
            current_action: a.current_action.clone(),
            plan,
            recent_memories: a.memory.recent_annotated(6),
            relationships,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct LocationView {
    pub index: usize,
    pub name: &'static str,
    pub emoji: &'static str,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Serialize)]
pub struct TownSnapshot {
    pub day: u32,
    pub time: String,
    pub locations: Vec<LocationView>,
    pub agents: Vec<AgentView>,
}

#[derive(Clone, Serialize)]
pub struct Event {
    pub id: usize,
    pub day: u32,
    pub time: String,
    pub kind: String,
    pub text: String,
}

pub struct SharedState {
    snapshot: TownSnapshot,
    events: VecDeque<Event>,
    next_id: usize,
}

impl SharedState {
    pub fn new() -> Self {
        let locations = town::LOCATIONS
            .iter()
            .enumerate()
            .map(|(index, l)| LocationView {
                index,
                name: l.name,
                emoji: l.emoji,
                x: l.x,
                y: l.y,
            })
            .collect();
        Self {
            snapshot: TownSnapshot {
                day: 1,
                time: "08:00".to_string(),
                locations,
                agents: Vec::new(),
            },
            events: VecDeque::new(),
            next_id: 0,
        }
    }

    pub fn snapshot(&self) -> TownSnapshot {
        self.snapshot.clone()
    }

    pub fn events_after(&self, after: usize) -> Vec<Event> {
        self.events
            .iter()
            .filter(|e| e.id > after)
            .cloned()
            .collect()
    }

    pub fn set_agents(&mut self, day: u32, hour: u8, agents: &[Agent]) {
        self.snapshot.day = day;
        self.snapshot.time = format!("{:02}:00", hour);
        self.snapshot.agents = agents.iter().map(AgentView::from).collect();
    }

    pub fn push(&mut self, mut event: Event) {
        event.id = self.next_id;
        self.next_id += 1;
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}
