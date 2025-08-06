use std::sync::LazyLock;

use hecs::{Entity, EntityRef, Satisfies, World};

use super::{Burning, Health, Hostile, Job, Party, Stats};

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
    hits: u8,
    randomized: bool,
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

    fn build(self) -> Effect {
        Effect::Damage(self.damage, self.target)
    }
}

impl Default for Damage {
    fn default() -> Self {
        Self {
            damage_type: DamageType::Physical,
            multiplier: 1.,
            hits: 1,
            randomized: false,
        }
    }
}

#[derive(Clone, Copy)]
enum Effect {
    Damage(Damage, EffectTarget),
    Buff(Buff, EffectTarget),
    Debuff(Debuff, EffectTarget),
}

impl Effect {
    fn damage() -> DamageBuilder {
        DamageBuilder::new()
    }

    fn damage_type(damage_type: DamageType) -> DamageBuilder {
        DamageBuilder::new().damage_type(damage_type)
    }

    fn damage_multiplier(multiplier: f32) -> DamageBuilder {
        DamageBuilder::new().multiplier(multiplier)
    }
}

#[derive(Clone)]
pub struct Skill {
    pub name: &'static str,
    target: PrimaryTarget,
    effects: Vec<Effect>,
    drain: Job,
    generate: Job,
    conditional: Option<ConditionalEffect>,
}

#[derive(Clone)]
struct ConditionalEffect {
    test: fn(EntityRef, EntityRef) -> bool,
    effects: Vec<Effect>,
    drain: Job,
    generate: Job,
}

impl Default for ConditionalEffect {
    fn default() -> Self {
        Self {
            test: is_burning,
            effects: vec![],
            drain: Job::None,
            generate: Job::None,
        }
    }
}

impl Skill {
    fn effect(&self, caster: EntityRef, target: EntityRef) -> Skill {
        if let Some(conditional) = &self.conditional {
            if (conditional.test)(caster, target) {
                return Skill {
                    conditional: None,
                    effects: if conditional.effects.is_empty() {
                        self.effects.clone()
                    } else {
                        conditional.effects.clone()
                    },
                    drain: if matches!(conditional.drain, Job::None) {
                        self.drain
                    } else {
                        conditional.drain
                    },
                    generate: if matches!(conditional.generate, Job::None) {
                        self.generate
                    } else {
                        conditional.generate
                    },
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

    pub fn apply(&self, caster: EntityRef, target: EntityRef) {
        let skill = self.effect(caster, target);
        let mut caster_query = caster.query::<(&mut Health, &Stats)>();
        let mut target_query = target.query::<(&mut Health, &Stats)>();
        for effect in skill.effects.iter() {
            match effect {
                Effect::Damage(effect_damage, effect_target) => {
                    if let Some((Health(caster_health), caster_stats)) = caster_query.get()
                        && let Some((Health(target_health), target_stats)) = target_query.get()
                    {
                        if matches!(effect_damage.damage_type, DamageType::Healing) {
                            let damage = target_stats.max_health as f32 * effect_damage.multiplier;
                            *target_health =
                                (*target_health + damage as u32).min(target_stats.max_health);
                        } else {
                            let mut damage = caster_stats.attack as f32 * effect_damage.multiplier;
                            damage *= (caster_stats.attack as f32 / target_stats.defense as f32)
                                .clamp(0.5, 1.);
                            *target_health = target_health.saturating_sub(damage as u32);
                        }
                    }
                }
                _ => (),
            }
        }
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            name: "Uknown Skill",
            target: PrimaryTarget::Any,
            effects: vec![Effect::damage().build()],
            drain: Job::None,
            generate: Job::None,
            conditional: None,
        }
    }
}

fn is_burning(_caster: EntityRef, target: EntityRef) -> bool {
    target.satisfies::<&Burning>()
}

pub static BASIC_ATTACK: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Basic Attack",
    target: PrimaryTarget::Hostile,
    conditional: Some(ConditionalEffect {
        test: is_burning,
        effects: vec![Effect::damage_multiplier(2.).build()],
        ..Default::default()
    }),
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
