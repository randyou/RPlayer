use super::batch::*;
use super::table::*;
use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_navigation_clamps_at_edges() {
        assert_eq!(table_target(0, 5, TableMove::Up, 3), Some(0));
        assert_eq!(table_target(4, 5, TableMove::Down, 3), Some(4));
        assert_eq!(table_target(2, 5, TableMove::Home, 3), Some(0));
        assert_eq!(table_target(2, 5, TableMove::End, 3), Some(4));
    }

    #[test]
    fn table_paging_uses_page_size_and_clamps() {
        assert_eq!(table_target(5, 20, TableMove::PageDown, 4), Some(9));
        assert_eq!(table_target(5, 20, TableMove::PageUp, 4), Some(1));
        assert_eq!(table_target(18, 20, TableMove::PageDown, 4), Some(19));
        assert_eq!(table_target(1, 3, TableMove::PageUp, 4), Some(0));
    }

    #[test]
    fn empty_table_has_no_target() {
        assert_eq!(table_target(0, 0, TableMove::Down, 5), None);
    }

    #[test]
    fn table_scroll_keeps_target_near_the_center() {
        assert_eq!(table_scroll_offset(0, 600.0, 50.0), 0.0);
        assert_eq!(table_scroll_offset(10, 600.0, 50.0), 225.0);
        assert_eq!(table_scroll_offset(2, 400.0, 50.0), 0.0);
    }

    #[test]
    fn batch_bar_has_three_states() {
        assert_eq!(batch_bar_mode(0), BatchBarMode::Empty);
        assert_eq!(batch_bar_mode(1), BatchBarMode::Small);
        assert_eq!(batch_bar_mode(3), BatchBarMode::Small);
        assert_eq!(
            batch_bar_mode(BATCH_LARGE_THRESHOLD - 1),
            BatchBarMode::Small
        );
        assert_eq!(batch_bar_mode(BATCH_LARGE_THRESHOLD), BatchBarMode::Large);
        assert_eq!(batch_bar_mode(24), BatchBarMode::Large);
    }
}
