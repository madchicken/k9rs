use gpui::*;

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

    pub fn into_element(self) -> Div {
        div()
            .flex()
            .w_full()
            .px_4()
            .py_2()
            .bg(rgb(0x313244))
            .gap_4()
            .items_center()
            // Logo / app name
            .child(
                div()
                    .text_color(rgb(0x89b4fa))
                    .child("k9rs")
            )
            // Context
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .child("ctx:")
                    )
                    .child(
                        div()
                            .text_color(rgb(0xa6e3a1))
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
                            .text_color(rgb(0x6c7086))
                            .child("ns:")
                    )
                    .child(
                        div()
                            .text_color(rgb(0xf9e2af))
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
                            .text_color(rgb(0x6c7086))
                            .child("res:")
                    )
                    .child(
                        div()
                            .text_color(rgb(0xf38ba8))
                            .child(self.resource)
                    )
            )
    }
}
