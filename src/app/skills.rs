use std::sync::LazyLock;

use hecs::{DynamicBundle, Entity, EntityRef, Satisfies, World};

use super::{Burning, Dead, Health, Hostile, Job, Party, Stats};

#[derive(Clone)]
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

#[derive(Clone)]
pub enum StatusEffect {
    Burning { stacks: u8, duration: u8 },
    Frozen { amount: u8 },
    Contagious { duration: u8 },
    Zapped { duration: u8 },
    Regen { amount: u32, duration: u8 },
    Stunned { duration: u8 },
    Revived,
    Cleansed,
}

#[derive(Clone)]
pub struct Skill {
    pub name: &'static str,
    damage_multiplier: f32,
    damage_type: DamageType,
    hits: u8,
    pub multi_target: bool,
    random_target: bool,
    target_friendly: bool,
    status_effects: Vec<StatusEffect>,
    drain: Job,
    generate: Job,
    conditional: Option<ConditionalEffect>,
}

#[derive(Clone)]
struct ConditionalEffect {
    test: fn(EntityRef, EntityRef) -> bool,
    effect: Box<Skill>,
}

impl Skill {
    fn effect(&self, caster: EntityRef, target: EntityRef) -> Skill {
        if let Some(conditional) = &self.conditional {
            if (conditional.test)(caster, target) {
                return Skill {
                    name: self.name,
                    conditional: None,
                    ..*conditional.effect.clone()
                };
            }
        }
        return self.clone();
    }

    pub fn get_targets(&self, world: &World) -> Vec<Entity> {
        world
            .query::<(Satisfies<&Party>, Satisfies<&Hostile>)>()
            .iter()
            .filter_map(|(entity, (friendly, hostile))| {
                if self.target_friendly && friendly {
                    Some(entity)
                } else if !self.target_friendly && hostile {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn apply(&self, caster: EntityRef, target: EntityRef) {
        let skill = self.effect(caster, target);
        let mut caster_query = caster.query::<&Stats>();
        let mut target_query = target.query::<(&mut Health, &Stats)>();
        if let Some(caster_stats) = caster_query.get()
            && let Some((Health(health), target_stats)) = target_query.get()
        {
            let mut damage = caster_stats.attack as f32 * skill.damage_multiplier;
            damage *= (caster_stats.attack as f32 / target_stats.defense as f32).clamp(0.5, 1.);
            *health = health.saturating_sub(damage as u32);
        }
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            name: "Uknown Skill",
            damage_multiplier: 1.,
            damage_type: DamageType::Physical,
            hits: 1,
            multi_target: false,
            random_target: false,
            target_friendly: false,
            status_effects: Vec::new(),
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
    conditional: Some(ConditionalEffect {
        test: is_burning,
        effect: Box::new(Skill {
            damage_multiplier: 2.,
            ..Default::default()
        }),
    }),
    ..Default::default()
});

pub static POTION: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Potion",
    damage_type: DamageType::Healing,
    target_friendly: true,
    ..Default::default()
});

pub static CLEANSE: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Cleanse",
    damage_type: DamageType::Healing,
    damage_multiplier: 0.,
    target_friendly: true,
    status_effects: vec![StatusEffect::Cleansed],
    ..Default::default()
});

pub static REVIVE: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Revive",
    damage_type: DamageType::Healing,
    target_friendly: true,
    status_effects: vec![StatusEffect::Revived],
    ..Default::default()
});
