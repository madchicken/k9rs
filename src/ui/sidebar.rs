use gpui::*;
use gpui_component::sidebar::{
    Sidebar as GpuiSidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
    SidebarMenuItem,
};
use gpui_component::{h_flex, IconName, Side, Sizable};

use crate::model::resources::RESOURCES;

/// Map a resource api_name to an appropriate icon
fn resource_icon(api_name: &str) -> IconName {
    match api_name {
        "pods" => IconName::SquareTerminal,
        "deployments" => IconName::ArrowUp,
        "statefulsets" => IconName::LayoutDashboard,
        "daemonsets" => IconName::Globe,
        "replicasets" => IconName::Copy,
        "jobs" => IconName::Loader,
        "cronjobs" => IconName::Calendar,
        "configmaps" => IconName::File,
        "secrets" => IconName::EyeOff,
        "serviceaccounts" => IconName::User,
        "services" => IconName::Globe,
        "ingresses" => IconName::ExternalLink,
        "persistentvolumes" => IconName::Folder,
        "persistentvolumeclaims" => IconName::FolderOpen,
        "namespaces" => IconName::Frame,
        "nodes" => IconName::Building2,
        "events" => IconName::Bell,
        _ => IconName::File,
    }
}

/// Build the sidebar using gpui-component Sidebar
pub fn build_sidebar(
    current_resource: &str,
    active_pf_count: usize,
    on_item_click: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    on_pf_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_item_click = std::rc::Rc::new(on_item_click);

    // Group resources by category
    let mut groups: Vec<(&str, Vec<(usize, &'static str, &'static str)>)> = vec![];
    let mut current_category = "";

    for (i, entry) in RESOURCES.iter().enumerate() {
        if entry.category != current_category {
            current_category = entry.category;
            groups.push((current_category, vec![]));
        }
        if let Some(group) = groups.last_mut() {
            group.1.push((i, entry.display_name, entry.api_name));
        }
    }

    let mut sidebar = GpuiSidebar::new(Side::Left).header(
        SidebarHeader::new().child(
            h_flex()
                .gap_2()
                .items_center()
                .child(Component::new(
                    gpui_component::Icon::new(IconName::Star)
                        .with_size(gpui_component::Size::Small),
                ))
                .child("k9rs"),
        ),
    );

    for (category, items) in groups {
        let mut menu = SidebarMenu::new();

        for (idx, display_name, api_name) in items {
            let is_active = api_name == current_resource;
            let icon = resource_icon(api_name);
            let cb = on_item_click.clone();

            menu = menu.child(
                SidebarMenuItem::new(display_name)
                    .icon(icon)
                    .active(is_active)
                    .on_click(move |ev, window, cx| {
                        cb(idx, ev, window, cx);
                    }),
            );
        }

        sidebar = sidebar.child(SidebarGroup::new(category).child(menu));
    }

    // Footer with port-forward count (clickable to open list)
    if active_pf_count > 0 {
        sidebar = sidebar.footer(
            SidebarFooter::new().child(
                div()
                    .id("pf-footer")
                    .cursor_pointer()
                    .on_click(move |ev, window, cx| on_pf_click(ev, window, cx))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Component::new(
                                gpui_component::Icon::new(IconName::ExternalLink)
                                    .with_size(gpui_component::Size::Small),
                            ))
                            .child(SharedString::from(format!(
                                "Port Forwards: {active_pf_count}"
                            ))),
                    ),
            ),
        );
    }

    Component::new(sidebar)
}
