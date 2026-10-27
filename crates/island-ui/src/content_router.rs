use island_core::models::IslandView;

pub struct ContentRouter;

impl ContentRouter {
    pub fn mode_to_index(mode: IslandView) -> i32 {
        match mode {
            IslandView::Compact => 0,
            IslandView::Wide => 1,
            IslandView::Expand => 2,
        }
    }
}
