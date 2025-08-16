use std::sync::LazyLock;

use super::*;

pub static BASIC_ATTACK: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Basic Attack",
    target: PrimaryTarget::Hostile,
    effects: vec![Effect::damage().build()],
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
