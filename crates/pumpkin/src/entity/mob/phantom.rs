use std::f64::consts::TAU;
use std::sync::{Arc, Weak};

use pumpkin_data::entity::EntityType;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    ai::{
        goal::{
            Controls, Goal, active_target::ActiveTargetGoal, look_around::RandomLookAroundGoal,
            look_at_entity::LookAtEntityGoal,
        },
        pathfinder::Navigator,
    },
    mob::{Mob, MobEntity},
};

const CIRCLE_RADIUS: f64 = 15.0;
const CIRCLE_VERTICAL_RADIUS: f64 = 3.0;
const CIRCLE_SPEED: f64 = 0.12;
const SWOOP_SPEED: f64 = 0.25;
const ATTACK_COOLDOWN: i32 = 80;
const ATTACK_DISTANCE_SQUARED: f64 = 2.5 * 2.5;

fn circle_position(
    anchor: Vector3<f64>,
    angle: f64,
    radius: f64,
    vertical_radius: f64,
) -> Vector3<f64> {
    Vector3::new(
        anchor.x + angle.cos() * radius,
        anchor.y + (angle * 0.5).sin() * vertical_radius,
        anchor.z + angle.sin() * radius,
    )
}

fn velocity_towards(from: Vector3<f64>, to: Vector3<f64>, speed: f64) -> Vector3<f64> {
    let delta = to - from;
    let distance = delta.length();
    if distance <= f64::EPSILON {
        Vector3::default()
    } else {
        delta.multiply(speed / distance, speed / distance, speed / distance)
    }
}

fn phantom_anchor(target: Vector3<f64>, rng: &mut impl RngExt) -> Vector3<f64> {
    Vector3::new(
        target.x + rng.random::<f64>().mul_add(16.0, -8.0),
        target.y + 12.0 + rng.random::<f64>() * 8.0,
        target.z + rng.random::<f64>().mul_add(16.0, -8.0),
    )
}

pub struct PhantomEntity {
    pub mob_entity: MobEntity,
}

impl PhantomEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        *mob_entity
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Navigator::flying();
        let phantom = Self { mob_entity };
        let mob_arc = Arc::new(phantom);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            goal_selector.add_goal(1, Box::new(PhantomAttackGoal::default()));
            goal_selector.add_goal(
                6,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 8.0),
            );
            goal_selector.add_goal(6, Box::new(RandomLookAroundGoal::default()));
        };

        {
            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            target_selector.add_goal(
                1,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PLAYER, true),
            );
        };

        mob_arc
    }
}

#[derive(Default)]
struct PhantomAttackGoal {
    anchor: Option<Vector3<f64>>,
    target: Option<Arc<dyn EntityBase>>,
    angle: f64,
    attack_cooldown: i32,
    swooping: bool,
}

impl PhantomAttackGoal {
    fn set_velocity_towards(mob: &dyn Mob, destination: Vector3<f64>, speed: f64) {
        let entity = mob.get_entity();
        let velocity = velocity_towards(entity.pos.load(), destination, speed);
        entity.velocity.store(velocity);

        if velocity.length_squared() > f64::EPSILON {
            entity
                .yaw
                .store((velocity.z.atan2(velocity.x).to_degrees() - 90.0) as f32);
            entity
                .pitch
                .store((-(velocity.y.atan2(velocity.x.hypot(velocity.z))).to_degrees()) as f32);
        }
    }

    fn reset_for_target(&mut self, mob: &dyn Mob, target: Arc<dyn EntityBase>) {
        let mut rng = mob.get_random();
        self.anchor = Some(phantom_anchor(target.get_entity().pos.load(), &mut rng));
        self.angle = rng.random::<f64>() * TAU;
        self.attack_cooldown = ATTACK_COOLDOWN;
        self.swooping = false;
        self.target = Some(target);
    }
}

impl Goal for PhantomAttackGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        let Some(target) = mob.get_mob_entity().get_target() else {
            return false;
        };
        if !target.get_entity().is_alive() {
            return false;
        }

        self.reset_for_target(mob, target);
        true
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        mob.get_mob_entity()
            .get_target()
            .is_some_and(|target| target.get_entity().is_alive())
    }

    fn start(&mut self, _mob: &dyn Mob) {}

    fn stop(&mut self, mob: &dyn Mob) {
        mob.get_entity().velocity.store(Vector3::default());
        self.anchor = None;
        self.target = None;
        self.swooping = false;
    }

    fn tick(&mut self, mob: &dyn Mob) {
        let Some(target) = mob.get_mob_entity().get_target() else {
            return;
        };
        if !target.get_entity().is_alive() {
            return;
        }

        if self
            .target
            .as_ref()
            .is_none_or(|current| current.get_entity().entity_id != target.get_entity().entity_id)
        {
            self.reset_for_target(mob, target.clone());
        }

        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }

        let target_pos = target.get_entity().pos.load();
        let mob_pos = mob.get_entity().pos.load();

        if self.swooping {
            if mob_pos.squared_distance_to_vec(&target_pos) <= ATTACK_DISTANCE_SQUARED {
                mob.get_mob_entity().try_attack(mob, target.as_ref());
                mob.on_attack(target.as_ref());
                self.swooping = false;
                self.attack_cooldown = ATTACK_COOLDOWN;
                let mut rng = mob.get_random();
                self.anchor = Some(phantom_anchor(target_pos, &mut rng));
                self.angle = rng.random::<f64>() * TAU;
            } else {
                Self::set_velocity_towards(mob, target_pos, SWOOP_SPEED);
            }
            return;
        }

        if self.attack_cooldown <= 0 {
            self.swooping = true;
            Self::set_velocity_towards(mob, target_pos, SWOOP_SPEED);
            return;
        }

        let Some(anchor) = self.anchor else {
            self.reset_for_target(mob, target);
            return;
        };

        self.angle = (self.angle + 0.04).rem_euclid(TAU);
        Self::set_velocity_towards(
            mob,
            circle_position(anchor, self.angle, CIRCLE_RADIUS, CIRCLE_VERTICAL_RADIUS),
            CIRCLE_SPEED,
        );
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }
}

impl Mob for PhantomEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_mob_gravity(&self) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{circle_position, velocity_towards};
    use pumpkin_util::math::vector3::Vector3;

    #[test]
    fn circle_position_keeps_the_requested_orbit_radius() {
        let anchor = Vector3::new(10.0, 30.0, -5.0);
        let position = circle_position(anchor, std::f64::consts::FRAC_PI_4, 12.0, 3.0);

        let horizontal_distance = (position.x - anchor.x).hypot(position.z - anchor.z);

        assert!((horizontal_distance - 12.0).abs() < f64::EPSILON);
        assert!((position.y - anchor.y).abs() <= 3.0);
    }

    #[test]
    fn velocity_towards_points_at_target_with_requested_speed() {
        let velocity = velocity_towards(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(3.0, 4.0, 0.0),
            0.25,
        );

        assert!((velocity.length() - 0.25).abs() < f64::EPSILON);
        assert!(velocity.x > 0.0);
        assert!(velocity.y > 0.0);
        assert!(velocity.z.abs() < f64::EPSILON);
    }
}
