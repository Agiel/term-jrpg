use std::sync::LazyLock;

use super::*;

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
