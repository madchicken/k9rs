use gpui::*;
use gpui_component::ActiveTheme;

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

    pub fn into_element(self, cx: &App) -> Div {
        let theme = cx.theme();
        let muted_bg = theme.muted;
        let primary = theme.primary;
        let foreground = theme.foreground;
        let success = theme.success;
        let warning = theme.warning;
        let muted_fg = theme.muted_foreground;

        let content = if self.command_mode {
            div()
                .flex()
                .gap_1()
                .child(div().text_color(warning).child(":"))
                .child(div().text_color(foreground).child(self.command_input))
        } else if self.filter_mode {
            div()
                .flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_color(primary)
                        .child("⌕"),
                )
                .child(
                    div()
                        .text_color(foreground)
                        .child(self.filter_text),
                )
        } else {
            let mut row = div()
                .flex()
                .gap_4()
                .child(div().text_color(success).child(self.message));

            // Show active filter indicator when not in filter mode but filter is set
            if !self.filter_text.is_empty() {
                row = row.child(
                    div()
                        .flex()
                        .gap_1()
                        .items_center()
                        .child(div().text_color(primary).child("⌕"))
                        .child(div().text_color(foreground).child(self.filter_text)),
                );
            }

            row = row.child(
                div()
                    .text_color(muted_fg)
                    .child("':' cmd | / filter | ↑↓ nav | Ctrl-N ns | Ctrl-K ctx | Ctrl-R res | Cmd-Q quit"),
            );

            row
        };

        div()
            .flex()
            .w_full()
            .px_4()
            .py_1()
            .bg(muted_bg)
            .child(content)
    }
}
