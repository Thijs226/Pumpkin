use std::sync::Arc;

use pumpkin_data::{Enchantment, entity::EntityType, item_stack::ItemStack};
use pumpkin_macros::pumpkin_block_from_tag;
use pumpkin_util::GameMode;

use crate::block::BlockBehaviour;
use crate::block::BrokenArgs;
use crate::entity::Entity;

#[pumpkin_block_from_tag("c:cobblestones/infested")]
pub struct InfestedBlock;

impl BlockBehaviour for InfestedBlock {
    fn broken(&self, args: BrokenArgs<'_>) {
        {
            // TODO: ugly fix, use onStacksDropped
            let held_item = args.player.inventory().held_item();
            if !should_spawn_silverfish(args.player.gamemode.load(), &held_item) {
                return;
            }
            let entity = Entity::new(
                args.world.clone(),
                args.position.0.to_f64(),
                &EntityType::SILVERFISH,
            );

            args.world.spawn_entity(Arc::new(entity));
        }
    }
}

fn should_spawn_silverfish(gamemode: GameMode, held_item: &ItemStack) -> bool {
    gamemode != GameMode::Creative && held_item.get_enchantment_level(&Enchantment::SILK_TOUCH) == 0
}

#[cfg(test)]
mod tests {
    use super::should_spawn_silverfish;
    use pumpkin_data::{Enchantment, item::Item, item_stack::ItemStack};
    use pumpkin_util::GameMode;

    #[test]
    fn silk_touch_prevents_silverfish_spawn() {
        let plain_tool = ItemStack::new(1, &Item::DIAMOND_PICKAXE);
        let mut silk_touch_tool = plain_tool.clone();
        silk_touch_tool.enchant(&Enchantment::SILK_TOUCH, 1);

        assert!(should_spawn_silverfish(GameMode::Survival, &plain_tool));
        assert!(!should_spawn_silverfish(
            GameMode::Survival,
            &silk_touch_tool
        ));
        assert!(!should_spawn_silverfish(GameMode::Creative, &plain_tool));
    }
}
