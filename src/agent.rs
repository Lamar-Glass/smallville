use crate::memory::MemoryStream;

#[derive(Clone)]
pub struct PlanEntry {
    pub hour: u8,
    pub location: usize,
    pub action: String,
}

pub struct Agent {
    pub id: usize,
    pub name: String,
    pub emoji: String,
    pub color: &'static str,
    pub bio: String,
    pub traits: String,
    pub home: usize,
    pub work: usize,
    pub location: usize,
    pub current_action: String,
    pub plan: Vec<PlanEntry>,
    pub memory: MemoryStream,
    pub relationships: std::collections::HashMap<String, i32>,
    pub last_plan_day: u32,
    pub reflections_today: u32,
}

impl Agent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: usize,
        name: &str,
        emoji: &str,
        color: &'static str,
        bio: &str,
        traits: &str,
        home: usize,
        work: usize,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            emoji: emoji.to_string(),
            color,
            bio: bio.to_string(),
            traits: traits.to_string(),
            home,
            work,
            location: home,
            current_action: "waking up".to_string(),
            plan: Vec::new(),
            memory: MemoryStream::new(),
            relationships: std::collections::HashMap::new(),
            last_plan_day: 0,
            reflections_today: 0,
        }
    }
}
