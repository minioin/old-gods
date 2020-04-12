use old_gods::{prelude::*, rendering::action};

use super::super::components::inventory::Inventory;
use super::super::systems::looting::Loot;


/// A renderable inventory item.
pub struct InventoryItem {
    pub name: String,
    pub frame: TextureFrame,
    pub usable: bool,
    pub count: usize,
}


/// A renderable inventory.
pub struct InventoryRendering {
    pub items: Vec<InventoryItem>,
    pub name: String,
}


/// A renderable loot operation.
pub struct LootRendering {
    pub inventory_a: InventoryRendering,
    pub inventory_b: Option<InventoryRendering>,
    pub cursor_in_a: bool,
    pub index: usize,
}


impl LootRendering {
    /// Return the item currently under the cursor
    pub fn current_item(&self) -> Option<&InventoryItem> {
        let inv = if self.inventory_b.is_none() || self.cursor_in_a {
            &self.inventory_a
        } else {
            &self.inventory_b.as_ref().unwrap()
        };
        inv.items.get(self.index)
    }
}


/// Draw a player inventory
pub fn draw_loot<Ctx: HasRenderingContext, Rsrc: Resources<<Ctx::Ctx as RenderingContext>::Image>>(
    context: &mut Ctx,
    resources: &mut Rsrc,
    point: &V2,
    loot: LootRendering,
) -> Result<(), String> {
    let item_height = 50;
    let name_height = 20;

    let mut invs = vec![(true, &loot.inventory_a)];
    if loot.inventory_b.is_some() {
        invs.push((false, &loot.inventory_b.as_ref().unwrap()));
    }

    let mut origin = V2::new(10.0, 10.0);
    for (is_a, inv) in invs {
        // Find the widest item to determine the width
        let mut longest_name = "";
        for item in inv.items.iter() {
            if longest_name.len() < item.name.len() {
                longest_name = &item.name;
            }
        }

        let longest_name_size = context.measure_text(&Ctx::fancy_text(longest_name))?;
        let width = 48.0 + longest_name_size.0 as f32 + 8.0;

        // Draw the background
        context.set_fill_color(&Color::rgba(0, 0, 0, 128));
        let bg_height = if inv.items.len() > 0 {
            inv.items.len() * item_height
        } else {
            item_height
        } + name_height;
        let bg_rect = AABB::new(origin.x, origin.y, width, bg_height as f32);
        context.fill_rect(&bg_rect);
        context.set_stroke_color(&Color::rgba(255, 255, 225, 255));
        context.stroke_rect(&bg_rect);

        // Draw each item
        for (item, n) in inv.items.iter().zip(0..inv.items.len()) {
            let pos = origin + V2::new(0.0, name_height as f32 + item_height as f32 * n as f32);
            resources
                .when_loaded(&item.frame.sprite_sheet, |tex| -> Result<(), String> {
                    let src = AABB::new(
                        item.frame.source_aabb.x as f32,
                        item.frame.source_aabb.y as f32,
                        item.frame.source_aabb.w as f32,
                        item.frame.source_aabb.h as f32,
                    );
                    let dest = AABB::new(
                        pos.x,
                        pos.y,
                        item.frame.size.0 as f32,
                        item.frame.size.1 as f32,
                    );
                    context.draw_sprite(
                        src,
                        dest,
                        item.frame.is_flipped_horizontally,
                        item.frame.is_flipped_vertically,
                        item.frame.is_flipped_diagonally,
                        tex,
                    )?;
                    let text_pos = pos + V2::new(48.0, 10.0);
                    let name = item.name.clone();
                    let text = Ctx::fancy_text(name.as_str());
                    context.draw_text(&text, &text_pos)?;
                    let item_aabb_size = context.measure_text(&text)?;
                    let item_aabb = AABB {
                        top_left: text_pos,
                        extents: V2::new(item_aabb_size.0, item_aabb_size.1),
                    };
                    if item.count > 1 {
                        let pos = V2::new(item_aabb.left() as f32, item_aabb.bottom() as f32 + 2.0);
                        let mut text = Ctx::normal_text(&format!("x{}", item.count));
                        text.font.size = 12;
                        context.draw_text(&text, &pos)?;
                    }
                    Ok(())
                })?
                .unwrap_or(Ok(()))?;
        }

        // Draw the inventory name
        let inv_name_text = Ctx::fancy_text(inv.name.as_str());
        context.draw_text(&inv_name_text, &(origin + V2::new(2.0, 2.0)))?;

        // Draw the cursor
        let looking_at_this_inv = loot.cursor_in_a == is_a;
        if looking_at_this_inv && inv.items.len() > 0 {
            let ndx = loot.index;
            context.set_stroke_color(&Color::rgb(0, 255, 0));
            let cursor_y = name_height as f32 + origin.y + ndx as f32 * 50.0;
            context.stroke_rect(&AABB {
                top_left: V2::new(origin.x + 1.0, cursor_y),
                extents: V2::new(width - 1.0, 50.0),
            });
        } else if inv.items.len() == 0 {
            // Draw the empty inventory
            let mut text = Ctx::fancy_text("(empty)");
            text.color = Color::rgb(128, 128, 128);
            context.draw_text(&text, &(origin + V2::new(45.0, 32.0)))?;
        }

        origin += V2::new(width, 0.0);
    }
    // Draw the close inventory msg
    let a_btn_rect = {
        let msg = Some("close".to_string());
        let items_len = usize::max(
            loot.inventory_a.items.len(),
            loot.inventory_b
                .as_ref()
                .map(|i| i.items.len())
                .unwrap_or(0),
        );
        let items_len = usize::max(1, items_len);
        let msg_y = item_height as f32 * items_len as f32 + name_height as f32;
        let msg_point = *point + V2::new(4.0, msg_y);
        action::draw_button::<Ctx>(context, ActionButton::Y, &msg_point, &msg)?
    };

    // Draw the "use" item inventory msg
    let current_item_is_usable = loot.current_item().map(|item| item.usable).unwrap_or(false);

    if current_item_is_usable {
        let msg = Some("use".to_string());
        let pos = V2::new(a_btn_rect.right() as f32, a_btn_rect.top() as f32);
        action::draw_button::<Ctx>(context, ActionButton::X, &pos, &msg)?;
    }

    Ok(())
}


pub fn make_loot_rendering<'s>(
    loot: &Loot,
    inventories: &ReadStorage<'s, Inventory>,
    names: &ReadStorage<'s, Name>,
) -> LootRendering {
    let mk_items = |inventory: &Inventory| -> Vec<InventoryItem> {
        let mut inv_items = vec![];
        for item in &inventory.items {
            let name = item.name.clone();
            let usable = item.usable;
            let count = item.stack.unwrap_or(1);
            let frame = item
                .rendering
                .as_frame()
                .expect("An item's Rendering is not a TextureFrame")
                .clone();
            inv_items.push(InventoryItem {
                name,
                frame,
                usable,
                count,
            });
        }
        inv_items
    };
    let mk_inv = |ent: Entity| {
        let Name(name) = names
            .get(ent)
            .expect("Cannot draw a loot without a Name")
            .clone();
        InventoryRendering {
            items: mk_items(
                inventories
                    .get(ent)
                    .expect("Cannot draw a loot without an Inventory"),
            ),
            name,
        }
    };
    let inventory_a = mk_inv(loot.ent_of_inventory_here);
    let inventory_b = loot.ent_of_inventory_there.map(mk_inv);
    LootRendering {
        inventory_a,
        inventory_b,
        cursor_in_a: loot.looking_here,
        index: loot.item_index,
    }
}