use std::sync::Arc;

use crate::entity::EntityBase;
use crate::entity::player::Player;
use crate::item::{ItemBehaviour, ItemMetadata};
use pumpkin_data::data_component_impl::CustomNameImpl;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;

pub struct NameTagItem;

impl ItemMetadata for NameTagItem {
    fn ids() -> Box<[u16]> {
        [Item::NAME_TAG.id].into()
    }
}

impl ItemBehaviour for NameTagItem {
    fn use_on_entity(&self, item: &mut ItemStack, player: &Player, entity: Arc<dyn EntityBase>) {
        let Some(living_entity) = entity.get_living_entity() else {
            return;
        };
        let target = entity.get_entity();
        if target.entity_type.saveable
            && let Some(name) = item.get_data_component::<CustomNameImpl>()
            && target.is_alive()
            && living_entity.health.load() > 0.0
        {
            target.set_custom_name(name.name.clone());
            if let Some(mob) = entity.get_mob() {
                mob.get_mob_entity().set_persistence_required();
            }
            item.decrement_unless_creative(player.gamemode.load(), 1);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
