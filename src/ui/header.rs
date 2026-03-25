use gpui::*;
use gpui_component::ActiveTheme;

/// Header bar showing cluster context, namespace, and current resource
pub struct Header {
    context: SharedString,
    namespace: SharedString,
    resource: SharedString,
}

impl Header {
    pub fn new(context: &str, namespace: &str, resource: &str) -> Self {
        Self {
            context: context.to_string().into(),
            namespace: namespace.to_string().into(),
            resource: resource.to_string().into(),
        }
    }

    pub fn into_element(self, cx: &App) -> Div {
        let theme = cx.theme();
        let muted_bg = theme.muted;
        let primary = theme.primary;
        let muted_fg = theme.muted_foreground;
        let success = theme.success;
        let warning = theme.warning;
        let danger = theme.danger;

        div()
            .flex()
            .w_full()
            .px_4()
            .py_2()
            .bg(muted_bg)
            .gap_4()
            .items_center()
            // Logo / app name
            .child(
                div()
                    .text_color(primary)
                    .child("k9rs")
            )
            // Context
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .text_color(muted_fg)
                            .child("ctx:")
                    )
                    .child(
                        div()
                            .text_color(success)
                            .child(self.context)
                    )
            )
            // Namespace
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .text_color(muted_fg)
                            .child("ns:")
                    )
                    .child(
                        div()
                            .text_color(warning)
                            .child(self.namespace)
                    )
            )
            // Resource type
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .text_color(muted_fg)
                            .child("res:")
                    )
                    .child(
                        div()
                            .text_color(danger)
                            .child(self.resource)
                    )
            )
    }
}
