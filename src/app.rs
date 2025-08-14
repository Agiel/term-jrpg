use rand::prelude::*;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
    sync::{LazyLock, Mutex},
    thread::sleep,
    time::Duration,
};

use hecs::{Entity, Satisfies, With, World};
use hecs_macros::Bundle;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    text::{Line, Span},
    widgets::{ListState, TableState},
};
use skills::Skill;

mod skills;

pub struct Log<'a> {
    lines: VecDeque<Line<'a>>,
}
pub static LOG: LazyLock<Mutex<Log>> = LazyLock::new(|| {
    Mutex::new(Log {
        lines: VecDeque::with_capacity(100),
    })
});

impl<'a> Log<'a> {
    pub fn write<'b>(&mut self, line: Line<'b>) {
        while self.lines.len() >= 100 {
            self.lines.pop_front();
        }
        // Deep clone to take ownership of the string
        let style = line.style;
        let alignment = line.alignment;
        let spans = line
            .into_iter()
            .map(|span| Span::styled(span.content.into_owned(), span.style))
            .collect::<Vec<_>>();
        self.lines.push_back(Line {
            style,
            alignment,
            spans,
        });
    }

    pub fn get_lines(&self) -> Vec<Line> {
        self.lines.iter().map(|s| s.clone()).collect()
    }
}

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
    Enemy,
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
#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub max_health: u32,
    pub attack: u32,
    pub speed: u32,
    pub crit: f32,
    pub evade: f32,
    pub defense: u32,
}

// Resources
#[derive(Clone, Copy, Default)]
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
    Think,
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
            stats.max_health = 80 + 20 * *level as u32;
            stats.attack = 16 + 4 * *level as u32;
            stats.speed = 100 + 20 * *level as u32;
            stats.crit = 0.1 + 0.05 * *level as f32;
            *health = stats.max_health;
        }
    }
}

fn spawn_party(world: &mut World) {
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

    level_up(world);
}

fn spawn_enemies(world: &mut World) {
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

    world.insert_one(rat, Burning(3)).unwrap();

    level_up(world);
}

impl App {
    pub fn new() -> App {
        let mut world = World::new();

        spawn_party(&mut world);

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
                                // self.start_targeting(&skills::STATIC_DISCHARGE);
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
                        self.apply_skill();
                        self.finish_turn();
                        if let Some(turn) = self.turn
                            && self.world.satisfies::<&Hostile>(turn).unwrap()
                        {
                            self.current_screen = CurrentScreen::Enemy;
                            return Some(Message::Think);
                        }
                    }
                    _ => (),
                },
                CurrentScreen::Enemy => match message {
                    Message::Think => {
                        sleep(Duration::from_secs(1));
                        self.think();
                        self.finish_turn();
                        if let Some(turn) = self.turn
                            && self.world.satisfies::<&Hostile>(turn).unwrap()
                        {
                            self.current_screen = CurrentScreen::Enemy;
                            return Some(Message::Think);
                        }
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
        None
    }

    fn think(&mut self) {
        self.skill = Some(&skills::BASIC_ATTACK);
        self.targets = self
            .world
            .query::<&Party>()
            .iter()
            .map(|(e, _)| e)
            .collect::<Vec<_>>();
        let mut rng = rand::rng();
        self.selected_target = Some(rng.random_range(..self.targets.len()));
        self.apply_skill();
    }

    fn apply_skill(&mut self) {
        let Some(skill) = self.skill else {
            return;
        };
        let targets = match self.selected_target {
            None => &self.targets,
            Some(selected) => &vec![self.targets[selected]],
        };
        skill.apply(&mut self.world, self.turn.unwrap(), targets);
        self.check_dead();
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
            let (targets, _) = skill.get_targets(&self.world);
            self.targets = targets;
            self.selected_target = (self.targets.len() > 0)
                .then_some(selected.clamp(0, self.targets.len().saturating_sub(1)));
        }
    }

    fn finish_turn(&mut self) {
        if self.world.query::<With<(), &Hostile>>().iter().count() == 0 {
            self.end_combat();
            return;
        }
        {
            let query = self
                .world
                .query_one::<(&mut Initiative, &Stats)>(self.turn.unwrap());
            // Entity may have died during its turn so we can't unwrap the Result here.
            if let Ok(mut query) = query
                && let Some((Initiative(initiative), stats)) = query.get()
            {
                *initiative += 1. / stats.speed as f32;
            }
        }
        self.refresh_next_up();
        if let Some(next_up) = &self.next_up {
            self.turn = next_up.0.peek().map(|i| i.entity);
        }
        self.current_screen = CurrentScreen::Main;
    }

    fn end_combat(&mut self) {
        level_up(&mut self.world);
        self.game_state = GameState::Overworld;
        self.current_screen = CurrentScreen::Main;

        // TODO: Until the overworld is implemented, just restart combat
        self.start_combat(Advantage::Neutral);
    }

    fn start_targeting(&mut self, skill: &'static Skill) {
        self.previous_screen.push(self.current_screen);
        self.current_screen = CurrentScreen::Target;

        let (targets, many) = skill.get_targets(&self.world);
        self.targets = targets;
        self.selected_target = (!many).then_some(0);
        self.skill = Some(skill);
    }

    pub fn start_combat(&mut self, advantage: Advantage) {
        self.game_state = GameState::Combat;
        self.current_screen = CurrentScreen::Main;
        self.previous_screen = Vec::new();

        spawn_enemies(&mut self.world);

        for (_, (stats, Initiative(initiative), hostile)) in
            self.world
                .query_mut::<(&Stats, &mut Initiative, Satisfies<&Hostile>)>()
        {
            *initiative = 1. / stats.speed as f32;
            if matches!(advantage, Advantage::Friendly) && hostile
                || matches!(advantage, Advantage::Enemy) && !hostile
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
                .query::<(&Initiative, &Stats, Satisfies<&Hostile>)>()
                .iter()
                .map(
                    |(entity, (&Initiative(initiative), stats, hostile))| InitiativeInfo {
                        initiative,
                        speed: stats.speed,
                        hostile,
                        entity,
                    },
                ),
        )));
    }
}
