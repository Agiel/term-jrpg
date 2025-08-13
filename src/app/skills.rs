use std::sync::{LazyLock, Mutex};

use hecs::{Entity, EntityRef, Satisfies, World};
use rand::prelude::*;
use ratatui::style::{Color, Style, Stylize};

use super::{Burning, Health, Hostile, Job, Name, Party, Stats, StyledLine, StyledSpan, log_write};

#[derive(Clone, Copy)]
pub enum DamageType {
    Physical,
    Healing,
    Fire,
    Ice,
    Toxic,
    Electrical,
    Dark,
    Light,
}

#[derive(Clone, Copy)]
pub enum Debuff {
    Burning { stacks: u8, duration: u8 },
    Frozen { amount: u8 },
    Contagious { duration: u8 },
    Zapped { duration: u8 },
    Regen { amount: u32, duration: u8 },
    Stunned { duration: u8 },
    Slow { duration: u8 },
    Confused { duration: u8 },
}

#[derive(Clone, Copy)]
pub enum Buff {
    Haste { duration: u8 },
    Revived,
    Cleansed,
}

#[derive(Clone, Copy)]
enum PrimaryTarget {
    Hostile,
    AllHostile,
    Friendly,
    AllFriendly,
    Any,
    All,
}

#[derive(Clone, Copy)]
enum EffectTarget {
    Target,
    Caster,
    Hostile,
    Friendly,
    All,
}

#[derive(Clone, Copy)]
struct Damage {
    damage_type: DamageType,
    multiplier: f32,
    crit_multiplier: f32,
    hits: u8,
    randomized: bool,
    modifier: Option<DamageModifier>,
}

impl Damage {
    fn get_modified(&self, caster: EntityRef, target: EntityRef) -> Self {
        if let Some(modifier) = self.modifier {
            if modifier.test.0(caster, target) {
                return Self {
                    damage_type: modifier.damage_type.unwrap_or(self.damage_type),
                    multiplier: modifier.multiplier.unwrap_or(self.multiplier),
                    crit_multiplier: modifier.crit_multiplier.unwrap_or(self.crit_multiplier),
                    ..self.clone()
                };
            }
        }
        self.clone()
    }
}

#[derive(Clone, Copy)]
struct TestFn(fn(caster: EntityRef, target: EntityRef) -> bool);

#[derive(Clone, Copy)]
struct DamageModifier {
    test: TestFn,
    damage_type: Option<DamageType>,
    multiplier: Option<f32>,
    crit_multiplier: Option<f32>,
}

impl Default for DamageModifier {
    fn default() -> Self {
        Self {
            test: TestFn(is_burning),
            damage_type: None,
            multiplier: None,
            crit_multiplier: None,
        }
    }
}

struct DamageBuilder {
    damage: Damage,
    target: EffectTarget,
}

impl DamageBuilder {
    fn new() -> Self {
        Self {
            damage: Default::default(),
            target: EffectTarget::Target,
        }
    }

    fn damage_type(mut self, damage_type: DamageType) -> Self {
        self.damage.damage_type = damage_type;
        self
    }

    fn multiplier(mut self, multiplier: f32) -> Self {
        self.damage.multiplier = multiplier;
        self
    }

    fn crit(mut self, crit: f32) -> Self {
        self.damage.crit_multiplier = crit;
        self
    }

    fn hits(mut self, hits: u8) -> Self {
        self.damage.hits = hits;
        self
    }

    fn randomized(mut self) -> Self {
        self.damage.randomized = true;
        self
    }

    fn target(mut self, target: EffectTarget) -> Self {
        self.target = target;
        self
    }

    fn modifier(mut self, modifier: DamageModifier) -> Self {
        self.damage.modifier = Some(modifier);
        self
    }

    fn build(self) -> Effect {
        Effect::Damage(self.damage, self.target)
    }
}

impl Default for Damage {
    fn default() -> Self {
        Self {
            damage_type: DamageType::Physical,
            multiplier: 1.,
            crit_multiplier: 1.5,
            hits: 1,
            randomized: false,
            modifier: None,
        }
    }
}

#[derive(Clone)]
enum Effect {
    Damage(Damage, EffectTarget),
    Buff(Buff, EffectTarget),
    Debuff(Debuff, EffectTarget),
    Gain(Job),
    Drain(Job),
    Conditional(TestFn, Vec<Effect>),
}

impl Effect {
    fn damage() -> DamageBuilder {
        DamageBuilder::new()
    }

    fn damage_type(damage_type: DamageType) -> DamageBuilder {
        DamageBuilder::new().damage_type(damage_type)
    }
}

#[derive(Clone)]
pub struct Skill {
    pub name: &'static str,
    target: PrimaryTarget,
    effects: Vec<Effect>,
    on_hit: Vec<Effect>,
    on_crit: Vec<Effect>,
    cost: Job,
    modifier: Option<SkillModifier>,
}

#[derive(Clone)]
struct SkillModifier {
    test: TestFn,
    effects: Option<Vec<Effect>>,
    on_hit: Option<Vec<Effect>>,
    on_crit: Option<Vec<Effect>>,
    cost: Option<Job>,
}

impl Default for SkillModifier {
    fn default() -> Self {
        Self {
            test: TestFn(is_burning),
            effects: None,
            on_hit: None,
            on_crit: None,
            cost: None,
        }
    }
}

impl Skill {
    fn get_modified(&self, caster: EntityRef) -> Skill {
        if let Some(modifier) = &self.modifier {
            // Test functions take both caster and target for reusability.
            if modifier.test.0(caster, caster) {
                return Skill {
                    modifier: None,
                    effects: modifier.effects.as_ref().unwrap_or(&self.effects).clone(),
                    on_hit: modifier.on_hit.as_ref().unwrap_or(&self.on_hit).clone(),
                    on_crit: modifier.on_crit.as_ref().unwrap_or(&self.on_crit).clone(),
                    cost: modifier.cost.unwrap_or(self.cost),
                    ..self.clone()
                };
            }
        }
        self.clone()
    }

    pub fn get_targets(&self, world: &World) -> (Vec<Entity>, bool) {
        (
            world
                .query::<(Satisfies<&Party>, Satisfies<&Hostile>)>()
                .iter()
                .filter_map(|(entity, (friendly, hostile))| match self.target {
                    PrimaryTarget::Hostile | PrimaryTarget::AllHostile if friendly => None,
                    PrimaryTarget::Friendly | PrimaryTarget::AllFriendly if hostile => None,
                    _ => Some(entity),
                })
                .collect(),
            matches!(
                self.target,
                PrimaryTarget::AllHostile | PrimaryTarget::AllFriendly | PrimaryTarget::All
            ),
        )
    }

    pub fn apply(&self, world: &mut World, caster: Entity, targets: &Vec<Entity>) {
        {
            let mut caster_query = world
                .query_one::<(&Name, Satisfies<&Hostile>)>(caster)
                .unwrap();
            let (Name(caster_name), hostile) = caster_query.get().unwrap();

            let color = if hostile { Color::Red } else { Color::Green };
            log_write(StyledLine::new(vec![
                StyledSpan::styled(caster_name, Style::new().fg(color)),
                StyledSpan::new(" uses "),
                StyledSpan::styled(self.name, Style::new().blue()),
            ]));
        }
        for effect in self.effects.iter() {
            self.effect(effect, world, caster, targets, true);
        }
    }

    fn effect(
        &self,
        effect: &Effect,
        world: &mut World,
        caster: Entity,
        targets: &Vec<Entity>,
        on_hit: bool,
    ) {
        match effect {
            Effect::Damage(effect_damage, effect_target) => {
                let targets = match effect_target {
                    EffectTarget::Target => targets,
                    EffectTarget::Caster => &vec![caster],
                    EffectTarget::Hostile => {
                        &world.query::<&Hostile>().iter().map(|(e, _)| e).collect()
                    }
                    EffectTarget::Friendly => {
                        &world.query::<&Party>().iter().map(|(e, _)| e).collect()
                    }
                    EffectTarget::All => &world.query::<&Health>().iter().map(|(e, _)| e).collect(),
                };
                let mut target_iter = targets.iter().cycle();

                let hits = if effect_damage.randomized {
                    // If randomized, hits is the total number of random hits
                    effect_damage.hits
                } else {
                    // otherwise it's hits per target
                    effect_damage.hits * targets.len() as u8
                };

                let mut rng = rand::rng();

                for _ in 0..hits {
                    let target = if effect_damage.randomized {
                        // len inclusive lets us hit the same target multiple times
                        target_iter.nth(rng.random_range(..=targets.len()))
                    } else {
                        target_iter.next()
                    };

                    let Some(&target) = target else {
                        break;
                    };

                    let effect_damage = {
                        let caster_ref = world.entity(caster).expect("Caster not found");
                        let target_ref = world.entity(target).expect("Target not found");
                        effect_damage.get_modified(caster_ref, target_ref)
                    };

                    let mut on_crit = false;

                    {
                        let caster_stats = world
                            .get::<&Stats>(caster)
                            .expect("Can't cast skills without a Stats component!");
                        let mut target_query = world
                            .query_one::<(&mut Health, &Stats, &Name, Satisfies<&Hostile>)>(target)
                            .expect("Target not found");
                        let (Health(target_health), target_stats, Name(target_name), hostile) =
                            target_query
                                .get()
                                .expect("Can't be a target without stats and health");

                        if matches!(effect_damage.damage_type, DamageType::Healing) {
                            let damage = target_stats.max_health as f32 * effect_damage.multiplier;
                            *target_health =
                                (*target_health + damage as u32).min(target_stats.max_health);
                        } else {
                            let mut damage = caster_stats.attack as f32;
                            damage *= (caster_stats.attack as f32 / target_stats.defense as f32)
                                .clamp(0.5, 1.);
                            damage *= effect_damage.multiplier;
                            if caster_stats.crit > rng.random() {
                                damage *= effect_damage.crit_multiplier;
                                on_crit = true;
                            }

                            *target_health = target_health.saturating_sub(damage as u32);

                            let color = if hostile { Color::Red } else { Color::Green };
                            let mut log_line = vec![
                                StyledSpan::styled(target_name, Style::new().fg(color)),
                                StyledSpan::new(" takes "),
                            ];
                            log_line.push(StyledSpan::styled(
                                &format!("{damage}"),
                                Style::default().bold(),
                            ));
                            if on_crit {
                                log_line
                                    .push(StyledSpan::styled(" critical", Style::default().bold()));
                            }
                            log_line.push(StyledSpan::new(" damage"));
                            log_write(StyledLine::new(log_line).right_aligned());
                        }
                    }

                    if on_hit {
                        let targets = vec![target];
                        for effect in self.on_hit.iter() {
                            self.effect(&effect, world, caster, &targets, false);
                        }
                        if on_crit {
                            for effect in self.on_crit.iter() {
                                self.effect(&effect, world, caster, &targets, false);
                            }
                        }
                    }
                }
            }
            Effect::Conditional(TestFn(test), effects) => {
                for target in targets.iter() {
                    let caster_ref = world.entity(caster).expect("Caster not found");
                    let target_ref = world.entity(*target).expect("Target not found");
                    if test(caster_ref, target_ref) {
                        for effect in effects.iter() {
                            self.effect(effect, world, caster, targets, on_hit);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            name: "Uknown Skill",
            target: PrimaryTarget::Any,
            effects: vec![Effect::damage().build()],
            on_hit: vec![],
            on_crit: vec![],
            cost: Job::None,
            modifier: None,
        }
    }
}

fn is_burning(_caster: EntityRef, target: EntityRef) -> bool {
    target.satisfies::<&Burning>()
}

pub static BASIC_ATTACK: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Basic Attack",
    target: PrimaryTarget::Hostile,
    effects: vec![
        Effect::damage()
            .modifier(DamageModifier {
                test: TestFn(is_burning),
                multiplier: Some(2.),
                ..Default::default()
            })
            .build(),
    ],
    ..Default::default()
});

pub static POTION: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Potion",
    // target: PrimaryTarget::Friendly,
    effects: vec![
        Effect::damage_type(DamageType::Healing)
            .multiplier(0.5)
            .build(),
    ],
    ..Default::default()
});

pub static STATIC_DISCHARGE: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Static Discharge",
    target: PrimaryTarget::AllHostile,
    effects: vec![
        Effect::damage_type(DamageType::Electrical)
            .hits(6)
            .randomized()
            .build(),
    ],
    on_crit: vec![
        Effect::damage_type(DamageType::Electrical)
            .randomized()
            .target(EffectTarget::Hostile)
            .build(),
    ],
    ..Default::default()
});

pub static CLEANSE: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Cleanse",
    target: PrimaryTarget::Friendly,
    effects: vec![Effect::Buff(Buff::Cleansed, EffectTarget::Target)],
    ..Default::default()
});

pub static REVIVE: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Revive",
    target: PrimaryTarget::Friendly,
    effects: vec![Effect::Buff(Buff::Revived, EffectTarget::Target)],
    ..Default::default()
});
