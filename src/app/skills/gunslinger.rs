use std::sync::LazyLock;

use super::*;

pub static RELOAD: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Reload",
    target: PrimaryTarget::Caster,
    effects: vec![Effect::Gain(Job::Gunslinger { ammo: u8::MAX })],
    ..Default::default()
});

pub static TACTICAL_RELOAD: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Tactical Reload",
    target: PrimaryTarget::Caster,
    effects: vec![
        Effect::Buff(Buff::Shell { duration: 1 }, EffectTarget::Target),
        Effect::Gain(Job::Gunslinger { ammo: u8::MAX }),
    ],
    cost: Job::Gunslinger { ammo: 1 },
    ..Default::default()
});

pub static DOUBLE_TAP: LazyLock<Skill> = LazyLock::new(|| Skill {
    name: "Double Tap",
    target: PrimaryTarget::Hostile,
    effects: vec![
        Effect::damage()
            .hits(2)
            .modifier(DamageModifier {
                test: TestFn(is_burning),
                multiplier: Some(1.5),
                ..Default::default()
            })
            .build(),
    ],
    cost: Job::Gunslinger { ammo: 2 },
    ..Default::default()
});
