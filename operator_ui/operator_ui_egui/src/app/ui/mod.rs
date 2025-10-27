
pub mod controls;
pub mod status;
pub mod camera;
pub mod diagnostics;
pub mod plot;
pub mod settings;



pub mod egui_tree {
    use std::fmt::Debug;
    use egui_tiles::{Container, ContainerKind, Tile, TileId, Tiles, Tree};
    use tracing::debug;

    // TODO create a PR to egui_tiles to add this method to `Tree`
    pub fn add_pane_to_root<Kind>(tree: &mut Tree<Kind>, new_kind: Kind, container_kind: ContainerKind) {
        if let Some(root_id) = tree.root() {
            let root = tree.tiles.remove(root_id).unwrap();
            match root {
                Tile::Pane(old_root_kind) => {
                    let new_root_pane_id = tree.tiles.insert_pane(old_root_kind);
                    let new_tile_id = tree.tiles.insert_pane(new_kind);
                    let children = vec![new_root_pane_id, new_tile_id];
                    let _new_root_container_id = tree.tiles.insert_container(Container::new(container_kind, children));
                    tree.root = Some(_new_root_container_id);
                }
                Tile::Container(mut container) => {
                    let new_tile_id = tree.tiles.insert_pane(new_kind);
                    container.add_child(new_tile_id);
                    tree.tiles.insert(root_id, Tile::Container(container));
                }
            }
        } else {
            tree.tiles = Tiles::default();
            let new_tile_id = tree.tiles.insert_pane(new_kind);
            tree.root = Some(new_tile_id);
        }
    }


    pub fn dump_tiles<Kind: Debug>(
        tiles: &mut Tiles<Kind>,
        tile_id: Option<TileId>,
    )
    {
        let Some(tile_id) = tile_id else {
            return
        };

        if let Some(tile) = tiles.remove(tile_id) {
            debug!("{:?}: {:?}", tile_id, tile);

            match &tile {
                Tile::Pane(_) => {}
                Tile::Container(container) => {
                    for &tile_id in container.children() {
                        dump_tiles(tiles, Some(tile_id));
                    }
                }
            }
            tiles.insert(tile_id, tile);
        }
    }
}