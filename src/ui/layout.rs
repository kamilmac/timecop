use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout configuration
pub struct AppLayout {
    pub breakpoint: u16,
    pub left_ratio: u16,
}

impl Default for AppLayout {
    fn default() -> Self {
        Self {
            breakpoint: 80,
            left_ratio: 30,
        }
    }
}

/// Computed layout areas
pub struct LayoutAreas {
    pub file_list: Rect,
    pub commit_list: Rect,
    pub preview: Rect,
    pub status_bar: Rect,
}

impl AppLayout {
    pub fn compute(&self, area: Rect) -> LayoutAreas {
        // Reserve space for status bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let main_area = main_chunks[0];
        let status_bar = main_chunks[1];

        if area.width >= self.breakpoint {
            // Wide layout: left column (files + commits) | right (preview)
            let h_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(self.left_ratio),
                    Constraint::Percentage(100 - self.left_ratio),
                ])
                .split(main_area);

            // Split left column into files and commits
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),
                    Constraint::Length(10), // commits window height
                ])
                .split(h_chunks[0]);

            LayoutAreas {
                file_list: left_chunks[0],
                commit_list: left_chunks[1],
                preview: h_chunks[1],
                status_bar,
            }
        } else {
            // Narrow layout: stacked
            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(20),
                    Constraint::Percentage(50),
                ])
                .split(main_area);

            LayoutAreas {
                file_list: v_chunks[0],
                commit_list: v_chunks[1],
                preview: v_chunks[2],
                status_bar,
            }
        }
    }
}

/// Calculate centered rect for modal
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
