use std::{cmp::Ordering, collections::BinaryHeap, sync::LazyLock};

use hecs::{Entity, World};
use hecs_macros::Bundle;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    widgets::{ListState, TableState},
};
use skills::Skill;

mod skills;

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

pub struct ActionListItem {
    pub text: &'static str,
    pub action: CurrentScreen, // TODO: Might need a separate enum but CurrentScreen is good for now
}

pub struct Consumable {
    pub name: &'static str,
    pub amount: u8,
    pub skill: &'static LazyLock<Skill>,
}

pub struct App {
    pub game_state: GameState,
    pub current_screen: CurrentScreen,
    pub previous_screen: Vec<CurrentScreen>,
    pub world: World,
    pub consumables: Vec<Consumable>,
    pub turn: Option<Entity>,
    pub next_up: Option<NextUp>,
    pub action_list_items: &'static [ActionListItem],
    pub action_list_state: ListState,
    pub consumable_list_state: TableState,
    pub targets: Vec<Entity>,
    pub selected_target: Option<usize>,
    pub skill: Option<&'static Skill>,
}

// Basic
#[derive(Default)]
pub struct Name(pub &'static str);
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
#[derive(Clone, Default)]
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
#[derive(Default)]
pub struct Initiative(pub f32);
pub struct Dead;

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
        //     name: Name("Gunslinger"),
        //     job: Job::Gunslinger { ammo: 12 },
        //     ..Default::default()
        // });
        // world.spawn(CharacterBundle {
        //     name: Name("Netrunner"),
        //     job: Job::Netrunner { ram: 16, heat: 54 },
        //     ..Default::default()
        // });
        world.spawn(CharacterBundle {
            name: Name("Technopriest"),
            job: Job::Technopriest { prayers: 4 },
            ..Default::default()
        });
        world.spawn(CharacterBundle {
            name: Name("Clairvoyant"),
            job: Job::Clairvoyant { sun: 0, moon: 0 },
            ..Default::default()
        });
        world.spawn(CharacterBundle {
            name: Name("Nanovampire"),
            job: Job::Nanovampire { battery: 100 },
            ..Default::default()
        });

        world.spawn(NPCBundle {
            name: Name("Sewer Rat"),
            ..Default::default()
        });
        world.spawn(NPCBundle {
            name: Name("Cybermutant"),
            ..Default::default()
        });
        let rat = world.spawn(NPCBundle {
            name: Name("Sewer Rat".into()),
            ..Default::default()
        });

        let _ = world.insert_one(rat, Burning(6));

        level_up(&mut world);

        let consumables = vec![
            Consumable {
                name: "Potion",
                amount: 15,
                skill: &skills::POTION,
            },
            Consumable {
                name: "Cleanse",
                amount: 3,
                skill: &skills::CLEANSE,
            },
            Consumable {
                name: "Revive",
                amount: 3,
                skill: &skills::REVIVE,
            },
        ];

        App {
            game_state: GameState::Combat,
            current_screen: CurrentScreen::Main,
            previous_screen: Vec::new(),
            world,
            consumables,
            turn: None,
            next_up: None,
            action_list_items: &[],
            action_list_state: Default::default(),
            consumable_list_state: TableState::default().with_selected(0),
            targets: Vec::new(),
            selected_target: None,
            skill: None,
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
                    self.previous_screen.push(self.current_screen);
                    self.current_screen = CurrentScreen::Exiting;
                    return None;
                }
            }
            Message::Cancel => {
                self.current_screen = self.previous_screen.pop().unwrap_or(CurrentScreen::Main)
            }
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
                            let next_screen = self.action_list_items[selected].action;
                            if matches!(next_screen, CurrentScreen::Target) {
                                self.start_targeting(&skills::BASIC_ATTACK);
                            } else {
                                self.previous_screen.push(self.current_screen);
                                self.current_screen = next_screen;
                            }
                        }
                    }
                    _ => (),
                },
                CurrentScreen::Skill => match message {
                    _ => (),
                },
                CurrentScreen::Item => match message {
                    Message::Up => {
                        if self.consumable_list_state.selected() == Some(0) {
                            self.consumable_list_state.select_last();
                        } else {
                            self.consumable_list_state.select_previous();
                        }
                    }
                    Message::Down => {
                        if self.consumable_list_state.selected() == Some(self.consumables.len() - 1)
                        {
                            self.consumable_list_state.select_first();
                        } else {
                            self.consumable_list_state.select_next();
                        }
                    }
                    Message::Select => {
                        if let Some(selected) = self.consumable_list_state.selected() {
                            let skill = self.consumables[selected].skill;
                            self.start_targeting(skill);
                        }
                    }
                    _ => (),
                },
                CurrentScreen::Target => match message {
                    Message::Up | Message::Left => {
                        if let Some(selected) = &mut self.selected_target {
                            *selected = (self.targets.len() + *selected - 1) % self.targets.len();
                        }
                    }
                    Message::Down | Message::Right => {
                        if let Some(selected) = &mut self.selected_target {
                            *selected = (*selected + 1) % self.targets.len();
                        }
                    }
                    Message::Select => {
                        let Some(skill) = self.skill else {
                            return None;
                        };
                        let caster = self.world.entity(self.turn.unwrap()).unwrap();
                        let targets = match self.selected_target {
                            None => &self.targets,
                            Some(selected) => &vec![self.targets[selected]],
                        };
                        targets.iter().for_each(|&target| {
                            let target = self.world.entity(target).unwrap();
                            skill.apply(caster, target);
                        });
                        self.check_dead();
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
        None
    }

    fn check_dead(&mut self) {
        let dead = self
            .world
            .query::<&Health>()
            .iter()
            .filter_map(|(entity, &Health(health))| (health == 0).then_some(entity))
            .collect::<Vec<_>>();
        dead.iter().for_each(|&entity| {
            if self.world.satisfies::<&Party>(entity).unwrap() {
                self.world.insert_one(entity, Dead).unwrap()
            } else {
                self.world.despawn(entity).unwrap();
            }
        });
        self.refresh_next_up();
        if let Some(skill) = self.skill
            && let Some(selected) = self.selected_target
        {
            self.targets = skill.get_targets(&self.world);
            self.selected_target = (self.targets.len() > 0)
                .then_some(selected.clamp(0, self.targets.len().saturating_sub(1)));
        }
    }

    fn start_targeting(&mut self, skill: &'static Skill) {
        self.previous_screen.push(self.current_screen);
        self.current_screen = CurrentScreen::Target;

        self.targets = skill.get_targets(&self.world);
        self.selected_target = (!skill.multi_target).then_some(0);
        self.skill = Some(skill);
    }

    pub fn start_combat(&mut self, advantage: Advantage) {
        self.game_state = GameState::Combat;
        self.current_screen = CurrentScreen::Main;
        self.previous_screen = Vec::new();

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
