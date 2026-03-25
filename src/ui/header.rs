use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::{ActiveTheme, IconName, Sizable};

/// Build the header bar with dropdown menus for context and resource,
/// and a button to open the namespace picker dialog.
pub fn build_header(
    context: &str,
    namespace: &str,
    resource: &str,
    contexts: &[String],
    on_context_change: std::rc::Rc<dyn Fn(String, &mut Window, &mut App)>,
    on_namespace_click: std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    on_resource_change: std::rc::Rc<dyn Fn(String, &mut Window, &mut App)>,
    cx: &App,
) -> Div {
    let theme = cx.theme();

    let current_ctx = context.to_string();
    let current_ns = namespace.to_string();
    let current_res = resource.to_string();
    let ctx_list: Vec<String> = contexts.to_vec();

    // Resource aliases for the resource dropdown
    let resource_list = vec![
        ("Pods", "pods"),
        ("Deployments", "deployments"),
        ("StatefulSets", "statefulsets"),
        ("DaemonSets", "daemonsets"),
        ("ReplicaSets", "replicasets"),
        ("Jobs", "jobs"),
        ("CronJobs", "cronjobs"),
        ("ConfigMaps", "configmaps"),
        ("Secrets", "secrets"),
        ("ServiceAccounts", "serviceaccounts"),
        ("Services", "services"),
        ("Ingresses", "ingresses"),
        ("PVs", "persistentvolumes"),
        ("PVCs", "persistentvolumeclaims"),
        ("Namespaces", "namespaces"),
        ("Nodes", "nodes"),
        ("Events", "events"),
    ];

    div()
        .flex()
        .w_full()
        .px_4()
        .py_1()
        .bg(theme.muted)
        .gap_3()
        .items_center()
        // Logo
        .child(
            div()
                .text_color(theme.primary)
                .font_weight(FontWeight::BOLD)
                .child("k9rs"),
        )
        // Context dropdown
        .child(build_dropdown(
            "ctx",
            &current_ctx,
            &ctx_list,
            IconName::Globe,
            on_context_change,
        ))
        // Namespace button — opens the namespace picker dialog
        .child(Component::new(
            Button::new("ns-picker-btn")
                .ghost()
                .small()
                .compact()
                .icon(IconName::Frame)
                .label(SharedString::from(current_ns))
                .on_click(move |ev, window, cx| {
                    on_namespace_click(ev, window, cx);
                }),
        ))
        // Resource dropdown
        .child({
            let on_change = on_resource_change;
            let label = resource_list
                .iter()
                .find(|(_, api)| *api == current_res.as_str())
                .map(|(name, _)| *name)
                .unwrap_or(&current_res);

            Component::new(
                Button::new("res-dropdown")
                    .ghost()
                    .small()
                    .compact()
                    .icon(IconName::LayoutDashboard)
                    .label(SharedString::from(label.to_string()))
                    .dropdown_menu(move |mut menu, _window, _cx| {
                        for (display, api_name) in &resource_list {
                            let api = api_name.to_string();
                            let cb = on_change.clone();
                            menu = menu.item(PopupMenuItem::new(*display).on_click(
                                move |_ev, window, cx| {
                                    cb(api.clone(), window, cx);
                                },
                            ));
                        }
                        menu
                    }),
            )
        })
}

fn build_dropdown(
    id: &str,
    current: &str,
    items: &[String],
    icon: IconName,
    on_change: std::rc::Rc<dyn Fn(String, &mut Window, &mut App)>,
) -> AnyElement {
    let current = current.to_string();
    let items: Vec<String> = items.to_vec();

    Component::new(
        Button::new(SharedString::from(format!("{id}-dropdown")))
            .ghost()
            .small()
            .compact()
            .icon(icon)
            .label(SharedString::from(current.clone()))
            .dropdown_menu(move |mut menu, _window, _cx| {
                for item in &items {
                    let value = item.clone();
                    let cb = on_change.clone();
                    let is_current = *item == current;
                    menu = menu.item(
                        PopupMenuItem::new(item.clone())
                            .checked(is_current)
                            .on_click(move |_ev, window, cx| {
                                cb(value.clone(), window, cx);
                            }),
                    );
                }
                menu
            }),
    )
    .into_any_element()
}
