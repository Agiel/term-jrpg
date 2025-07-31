use hecs::{Entity, World};
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
    pub speed: u8,
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
            *health = stats.max_health;
        }
    }
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
