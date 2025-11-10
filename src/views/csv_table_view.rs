use gpui::*;

/// A view for displaying CSV data in a table format
pub struct CsvTableView {
    column_headers: Vec<SharedString>,
    rows: Vec<Vec<SharedString>>,
}

impl CsvTableView {
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        // Sample data for demonstration
        let column_headers = vec![
            SharedString::from("Name"),
            SharedString::from("Age"),
            SharedString::from("City"),
            SharedString::from("Country"),
        ];

        let rows = vec![
            vec![
                SharedString::from("Alice"),
                SharedString::from("30"),
                SharedString::from("New York"),
                SharedString::from("USA"),
            ],
            vec![
                SharedString::from("Bob"),
                SharedString::from("25"),
                SharedString::from("London"),
                SharedString::from("UK"),
            ],
            vec![
                SharedString::from("Charlie"),
                SharedString::from("35"),
                SharedString::from("Paris"),
                SharedString::from("France"),
            ],
            vec![
                SharedString::from("Diana"),
                SharedString::from("28"),
                SharedString::from("Berlin"),
                SharedString::from("Germany"),
            ],
            vec![
                SharedString::from("Eve"),
                SharedString::from("32"),
                SharedString::from("Tokyo"),
                SharedString::from("Japan"),
            ],
        ];

        Self {
            column_headers,
            rows,
        }
    }

    fn render_header(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .bg(rgb(0xe0e0e0))
            .border_b_1()
            .border_color(rgb(0xcccccc))
            .children(
                self.column_headers.iter().map(|header| {
                    div()
                        .w(px(150.0))
                        .h(px(40.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_r_1()
                        .border_color(rgb(0xcccccc))
                        .font_weight(FontWeight::BOLD)
                        .child(header.clone())
                })
            )
    }

    fn render_row(&self, row: &[SharedString]) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .bg(rgb(0xf5f5f5))
            .border_b_1()
            .border_color(rgb(0xe0e0e0))
            .children(
                row.iter().map(|cell| {
                    div()
                        .w(px(150.0))
                        .h(px(30.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_r_1()
                        .border_color(rgb(0xe0e0e0))
                        .child(cell.clone())
                })
            )
    }
}

impl Render for CsvTableView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .child(self.render_header())
            .children(
                self.rows.iter().map(|row| self.render_row(row))
            )
    }
}
