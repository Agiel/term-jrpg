use std::{
    cmp::Ordering,
    collections::{BTreeSet, BinaryHeap},
};

use hecs::{Entity, With, World};
use hecs_macros::Bundle;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    widgets::ListState,
};

pub enum GameState {
    Menu,
    Overworld,
    Combat,
}

#[derive(Clone, Copy)]
pub enum CurrentScreen {
    Main,
    Skill,
    Target,
    Item,
    Exiting,
}

pub struct ActionListItem<'a> {
    pub text: &'a str,
    pub action: CurrentScreen, // TODO: Might need a separate enum but CurrentScreen is good for now
}

pub struct App {
    pub game_state: GameState,
    pub current_screen: CurrentScreen,
    pub world: World,
    pub turn: Option<Entity>,
    pub next_up: Option<NextUp>,
    pub action_list_items: &'static [ActionListItem<'static>],
    pub action_list_state: ListState,
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
    Clairvoyant {
        sun: u8,
        moon: u8,
    },
    Nanovampire {
        battery: u8,
    },
}

// Misc
#[derive(Default)]
pub struct Party;
#[derive(Default)]
pub struct Hostile;
pub struct Target;
#[derive(Default)]
pub struct Initiative(pub f32);

// Status
pub struct Burning(pub u8);
pub struct Frozen;
pub struct Confused;
pub struct Blind;
pub struct Stunned;

#[derive(Clone, PartialEq)]
pub struct InitiativeInfo {
    pub initiative: f32,
    pub speed: u32,
    pub hostile: bool,
    pub entity: Entity,
}

impl Eq for InitiativeInfo {}

impl Ord for InitiativeInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        let res = other
            .initiative
            .partial_cmp(&self.initiative)
            .unwrap_or(Ordering::Equal); // Reversed because BinaryHeap is a max heap
        if matches!(res, Ordering::Equal) {
            if self.hostile == other.hostile {
                other.entity.cmp(&self.entity)
            } else if self.hostile {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        } else {
            res
        }
    }
}

impl PartialOrd for InitiativeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct NextUp(pub BinaryHeap<InitiativeInfo>);

impl Iterator for NextUp {
    type Item = InitiativeInfo;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.0.pop();
        if let Some(i) = &item {
            self.0.push(InitiativeInfo {
                initiative: i.initiative + 1. / i.speed as f32,
                ..*i
            });
        }
        item
    }
}

pub enum Advantage {
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
    Select,
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
    initiative: Initiative,
    party: Party,
}

#[derive(Bundle, Default)]
struct NPCBundle {
    name: Name,
    health: Health,
    level: Level,
    xp: Xp,
    stats: Stats,
    initiative: Initiative,
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
        world.spawn(CharacterBundle {
            name: Name("Technopriest".into()),
            job: Job::Technopriest { prayers: 4 },
            ..Default::default()
        });
        world.spawn(CharacterBundle {
            name: Name("Clairvoyant".into()),
            job: Job::Clairvoyant { sun: 0, moon: 0 },
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
            turn: None,
            next_up: None,
            action_list_items: &[],
            action_list_state: Default::default(),
        }
    }

    pub fn handle_key(&self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Esc => Some(Message::Cancel),
            KeyCode::Up => Some(Message::Up),
            KeyCode::Down => Some(Message::Down),
            KeyCode::Left => Some(Message::Left),
            KeyCode::Right => Some(Message::Right),
            KeyCode::Enter => Some(Message::Select),
            _ => None,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Message> {
        match message {
            Message::Quit => {
                if matches!(self.current_screen, CurrentScreen::Exiting) {
                    return Some(Message::Quit);
                } else {
                    self.current_screen = CurrentScreen::Exiting;
                    return None;
                }
            }
            Message::Cancel => match self.current_screen {
                CurrentScreen::Exiting => {
                    self.current_screen = CurrentScreen::Main;
                    return None;
                }
                _ => (),
            },
            _ => (),
        }

        match self.game_state {
            GameState::Combat => match self.current_screen {
                CurrentScreen::Main => match message {
                    Message::Up => {
                        if self.action_list_state.selected() == Some(0) {
                            self.action_list_state.select_last();
                        } else {
                            self.action_list_state.select_previous();
                        }
                    }
                    Message::Down => {
                        if self.action_list_state.selected()
                            == Some(self.action_list_items.len() - 1)
                        {
                            self.action_list_state.select_first();
                        } else {
                            self.action_list_state.select_next();
                        }
                    }
                    Message::Select => {
                        if let Some(selected) = self.action_list_state.selected() {
                            self.current_screen = self.action_list_items[selected].action;
                        }
                    }
                    _ => (),
                },
                CurrentScreen::Skill => match message {
                    Message::Cancel => self.current_screen = CurrentScreen::Main,
                    _ => (),
                },
                CurrentScreen::Item => match message {
                    Message::Cancel => self.current_screen = CurrentScreen::Main,
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
        None
    }
    pub fn start_combat(&mut self, advantage: Advantage) {
        for (_, (stats, Initiative(initiative), hostile)) in
            self.world
                .query_mut::<(&Stats, &mut Initiative, Option<&Hostile>)>()
        {
            *initiative = 1. / stats.speed as f32;
            if matches!(advantage, Advantage::Friendly) && hostile.is_some()
                || matches!(advantage, Advantage::Enemy) && hostile.is_none()
            {
                *initiative *= 2.;
            }
        }
        self.refresh_next_up();
        self.turn = self
            .next_up
            .as_ref()
            .and_then(|nu| nu.0.peek().and_then(|i| Some(i.entity)));

        self.action_list_items = &[
            ActionListItem {
                text: "Skill",
                action: CurrentScreen::Skill,
            },
            ActionListItem {
                text: "Melee",
                action: CurrentScreen::Target,
            },
            ActionListItem {
                text: "Item",
                action: CurrentScreen::Item,
            },
        ];
        self.action_list_state.select_first();
    }

    fn refresh_next_up(&mut self) {
        self.next_up = Some(NextUp(BinaryHeap::from_iter(
            self.world
                .query::<(&Initiative, &Stats, Option<&Hostile>)>()
                .iter()
                .map(
                    |(entity, (&Initiative(initiative), stats, hostile))| InitiativeInfo {
                        initiative,
                        speed: stats.speed,
                        hostile: hostile.is_some(),
                        entity,
                    },
                ),
        )));
    }
}
