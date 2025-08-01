use std::cmp::Ordering;

use hecs::{Entity, With, World};
use hecs_macros::Bundle;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

pub enum GameState {
    Menu,
    Overworld,
    Combat,
}

pub enum CurrentScreen {
    Main,
    Skill,
    Target,
    Exiting,
}

pub struct App {
    pub game_state: GameState,
    pub current_screen: CurrentScreen,
    pub world: World,
    pub turn: Option<Entity>,
}

// Basic
#[derive(Default)]
pub struct Name(pub String);
#[derive(Default)]
pub struct Xp(pub u32);
#[derive(Default)]
pub struct Level(pub u8);
#[derive(Default)]
pub struct Health(pub u32);

// Stats
#[derive(Default)]
pub struct Stats {
    pub max_health: u32,
    pub speed: u32,
    pub crit: f32,
    pub evade: f32,
    pub defense: f32,
}

// Resources
#[derive(Default)]
pub enum Job {
    #[default]
    None,
    Gunslinger {
        ammo: u8,
    },
    Netrunner {
        ram: u8,
        heat: u8,
    },
    Technopriest {
        prayers: u8,
    },
    Oracle {
        sun: u8,
        moon: u8,
    },
    Nanovampire {
        battery: u8,
    },
}

// Status
#[derive(Default)]
pub struct Party;
#[derive(Default)]
pub struct Hostile;
pub struct Target;
pub struct Burning(pub u8);
pub struct Frozen;
pub struct Confused;
pub struct Blind;
pub struct Stunned;

#[derive(Clone, Copy)]
pub struct Order {
    pub turn: u8,
    pub speed: u32,
    pub offset: f32,
    pub friendly: bool,
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Equal)
    }
}

impl Eq for Order {}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.turn as f32 / self.speed as f32 + self.offset;
        let b = other.turn as f32 / other.speed as f32 + other.offset;
        let res = b.partial_cmp(&a); // Order flipped because binary_heap is a max heap
        if matches!(res, None) || matches!(res, Some(Ordering::Equal)) {
            if self.friendly == other.friendly {
                Ordering::Equal
            } else if self.friendly {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else {
            res.unwrap()
        }
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

enum TurnPriority {
    Friendly,
    Enemy,
    Neutral,
}

pub enum Message {
    Up,
    Down,
    Left,
    Right,
    Prev,
    Next,
    Cancel,
    Quit,
}

#[derive(Bundle, Default)]
struct CharacterBundle {
    name: Name,
    job: Job,
    health: Health,
    level: Level,
    xp: Xp,
    stats: Stats,
    party: Party,
}

#[derive(Bundle, Default)]
struct NPCBundle {
    name: Name,
    health: Health,
    level: Level,
    xp: Xp,
    stats: Stats,
    hostile: Hostile,
}

const LEVEL_THRESHOLDS: [u32; 10] = [0, 100, 300, 600, 1000, 1500, 2100, 2800, 3600, 4500];

fn level_up(world: &mut World) {
    for (_, (Level(level), &Xp(xp), stats, Health(health))) in
        world.query_mut::<(&mut Level, &Xp, &mut Stats, &mut Health)>()
    {
        if xp >= LEVEL_THRESHOLDS[*level as usize] {
            *level += 1;
            stats.max_health = 20 + 10 * *level as u32;
            stats.speed = 100 + 20 * *level as u32;
            *health = stats.max_health;
        }
    }
}

fn start_combat(priority: TurnPriority, world: &mut World) {
    let mut to_insert = Vec::new();
    for (entity, stats) in world.query::<With<&Stats, &Party>>().iter() {
        to_insert.push((
            entity,
            Order {
                speed: stats.speed,
                offset: 0.,
                turn: if matches!(priority, TurnPriority::Enemy) {
                    2
                } else {
                    1
                },
                friendly: true,
            },
        ))
    }
    for (entity, stats) in world.query::<With<&Stats, &Hostile>>().iter() {
        to_insert.push((
            entity,
            Order {
                speed: stats.speed,
                offset: 0.,
                turn: if matches!(priority, TurnPriority::Friendly) {
                    2
                } else {
                    1
                },
                friendly: false,
            },
        ))
    }
    to_insert.into_iter().for_each(|(entity, order)| {
        world.insert_one(entity, order).unwrap();
    });
}

impl App {
    pub fn new() -> App {
        let mut world = World::new();

        // world.spawn(CharacterBundle {
        //     name: Name("Gunslinger".into()),
        //     job: Job::Gunslinger { ammo: 12 },
        //     ..Default::default()
        // });
        // world.spawn(CharacterBundle {
        //     name: Name("Netrunner".into()),
        //     job: Job::Netrunner { ram: 16, heat: 54 },
        //     ..Default::default()
        // });
        let turn = world.spawn(CharacterBundle {
            name: Name("Technopriest".into()),
            job: Job::Technopriest { prayers: 4 },
            ..Default::default()
        });
        world.spawn(CharacterBundle {
            name: Name("Oracle".into()),
            job: Job::Oracle { sun: 0, moon: 0 },
            ..Default::default()
        });
        world.spawn(CharacterBundle {
            name: Name("Nanovampire".into()),
            job: Job::Nanovampire { battery: 100 },
            ..Default::default()
        });

        world.spawn(NPCBundle {
            name: Name("Sewer Rat".into()),
            ..Default::default()
        });
        world.spawn(NPCBundle {
            name: Name("Cybermutant".into()),
            ..Default::default()
        });
        let rat = world.spawn(NPCBundle {
            name: Name("Sewer Rat".into()),
            ..Default::default()
        });

        let _ = world.insert_one(rat, Burning(6));

        level_up(&mut world);

        start_combat(TurnPriority::Neutral, &mut world);

        App {
            game_state: GameState::Combat,
            current_screen: CurrentScreen::Main,
            world,
            turn: Some(turn),
        }
    }

    pub fn handle_key(&self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Esc => Some(Message::Cancel),
            _ => None,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Message> {
        match message {
            Message::Quit => {
                if matches!(self.current_screen, CurrentScreen::Exiting) {
                    Some(Message::Quit)
                } else {
                    self.current_screen = CurrentScreen::Exiting;
                    None
                }
            }
            Message::Cancel => match self.current_screen {
                CurrentScreen::Exiting => {
                    self.current_screen = CurrentScreen::Main;
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }
}
