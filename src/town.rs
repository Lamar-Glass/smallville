pub struct Location {
    pub name: &'static str,
    pub emoji: &'static str,
    pub x: i32,
    pub y: i32,
}

pub const LOCATIONS: &[Location] = &[
    Location {
        name: "The Lin Family House",
        emoji: "🏡",
        x: 16,
        y: 38,
    },
    Location {
        name: "Hobbit Hole Café",
        emoji: "☕",
        x: 60,
        y: 28,
    },
    Location {
        name: "Willow Market",
        emoji: "🏪",
        x: 48,
        y: 60,
    },
    Location {
        name: "The Park",
        emoji: "🌳",
        x: 28,
        y: 18,
    },
    Location {
        name: "Smallville Library",
        emoji: "📚",
        x: 74,
        y: 52,
    },
    Location {
        name: "City Hall",
        emoji: "🏛️",
        x: 70,
        y: 74,
    },
];

pub fn find(name: &str) -> Option<usize> {
    let needle = name.to_lowercase();
    LOCATIONS
        .iter()
        .position(|l| needle.contains(&l.name.to_lowercase()))
}

pub fn name(idx: usize) -> &'static str {
    LOCATIONS[idx].name
}

pub fn emoji(idx: usize) -> &'static str {
    LOCATIONS[idx].emoji
}
