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
            left_ratio: 20,
        }
    }
}

/// Computed layout areas
#[derive(Clone)]
pub struct LayoutAreas {
    pub header: Rect,
    pub file_list: Rect,
    pub pr_info: Rect,
    pub preview: Rect,
    pub status_bar: Rect,
}

impl AppLayout {
    pub fn compute(&self, area: Rect, pr_count: usize) -> LayoutAreas {
        // PR panel height: fits all PRs, max 16 visible
        let pr_height = match pr_count {
            0 => 3,                                        // border + "No open PRs"
            _ => ((pr_count + 2) as u16).min(18),          // border + count, max 16 visible
        };

        // Split: header | main content | PR panel | status bar
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(pr_height),
                Constraint::Length(1),
            ])
            .split(area);

        let header = v_chunks[0];
        let main_area = v_chunks[1];
        let pr_info = v_chunks[2];
        let status_bar = v_chunks[3];

        if area.width >= self.breakpoint {
            // Wide layout: file list | preview
            let left_width = ((main_area.width as u32 * self.left_ratio as u32) / 100)
                .clamp(40, 64) as u16;
            let h_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(left_width),
                    Constraint::Min(0),
                ])
                .split(main_area);

            LayoutAreas {
                header,
                file_list: h_chunks[0],
                pr_info,
                preview: h_chunks[1],
                status_bar,
            }
        } else {
            // Narrow layout: stacked
            let v_main = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                ])
                .split(main_area);

            LayoutAreas {
                header,
                file_list: v_main[0],
                pr_info,
                preview: v_main[1],
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
