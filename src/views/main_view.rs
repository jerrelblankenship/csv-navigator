use gpui::*;
use super::CsvTableView;

/// Main view for the CSV Navigator application
pub struct MainView {
    status_message: SharedString,
    csv_table: View<CsvTableView>,
}

impl MainView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let csv_table = cx.new_view(|cx| CsvTableView::new(cx));

        Self {
            status_message: SharedString::from("Ready"),
            csv_table,
        }
    }

    fn open_file(&mut self, _cx: &mut ViewContext<Self>) {
        self.status_message = SharedString::from("Open file clicked (not implemented)");
    }

    fn save_file(&mut self, _cx: &mut ViewContext<Self>) {
        self.status_message = SharedString::from("Save file clicked (not implemented)");
    }
}

impl Render for MainView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .child(
                // Toolbar
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .bg(rgb(0xf0f0f0))
                    .border_b_1()
                    .border_color(rgb(0xcccccc))
                    .child(
                        div()
                            .id("open-button")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x007acc))
                            .text_color(rgb(0xffffff))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x005a9e)))
                            .child("Open CSV")
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, cx| {
                                this.open_file(cx);
                                cx.notify();
                            }))
                    )
                    .child(
                        div()
                            .id("save-button")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x007acc))
                            .text_color(rgb(0xffffff))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x005a9e)))
                            .child("Save")
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, cx| {
                                this.save_file(cx);
                                cx.notify();
                            }))
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(0x007acc))
                            .ml_4()
                            .child(self.status_message.clone())
                    )
            )
            .child(
                // Content area with CSV table
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.csv_table.clone())
            )
    }
}
