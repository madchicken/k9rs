use gpui::*;

/// Bottom status bar — shows connection status, command input, or filter input
pub struct StatusBar {
    message: SharedString,
    command_mode: bool,
    command_input: SharedString,
    filter_mode: bool,
    filter_text: SharedString,
}

impl StatusBar {
    pub fn new(
        message: &str,
        command_mode: bool,
        command_input: &str,
        filter_mode: bool,
        filter_text: &str,
    ) -> Self {
        Self {
            message: message.to_string().into(),
            command_mode,
            command_input: command_input.to_string().into(),
            filter_mode,
            filter_text: filter_text.to_string().into(),
        }
    }

    pub fn into_element(self) -> Div {
        let content = if self.command_mode {
            div()
                .flex()
                .gap_1()
                .child(
                    div()
                        .text_color(rgb(0xf9e2af))
                        .child(":"),
                )
                .child(
                    div()
                        .text_color(rgb(0xcdd6f4))
                        .child(self.command_input),
                )
        } else if self.filter_mode {
            div()
                .flex()
                .gap_1()
                .child(
                    div()
                        .text_color(rgb(0x89b4fa))
                        .child("/"),
                )
                .child(
                    div()
                        .text_color(rgb(0xcdd6f4))
                        .child(self.filter_text),
                )
        } else {
            let mut row = div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .text_color(rgb(0xa6e3a1))
                        .child(self.message),
                );

            // Show active filter indicator when not in filter mode but filter is set
            if !self.filter_text.is_empty() {
                row = row.child(
                    div()
                        .flex()
                        .gap_1()
                        .child(div().text_color(rgb(0x89b4fa)).child("/"))
                        .child(div().text_color(rgb(0xcdd6f4)).child(self.filter_text)),
                );
            }

            row = row.child(
                div()
                    .text_color(rgb(0x6c7086))
                    .child("':' cmd | / filter | j/k nav | Ctrl-N ns | Cmd-Q quit"),
            );

            row
        };

        div()
            .flex()
            .w_full()
            .px_4()
            .py_1()
            .bg(rgb(0x313244))
            .child(content)
    }
}
