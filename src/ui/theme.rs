use gpui::{App, Hsla};
use gpui_component::theme::ActiveTheme;

/// Pre-resolved theme colors for use in UI components that don't have access to `App`.
#[derive(Clone)]
pub struct PanelColors {
    pub background: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub border: Hsla,
    pub selection: Hsla,
    pub primary: Hsla,
    pub secondary_foreground: Hsla,
    pub success: Hsla,
    pub danger: Hsla,
    pub danger_hover: Hsla,
    pub warning: Hsla,
    pub list_even: Hsla,
    pub overlay: Hsla,
    pub link: Hsla,
    pub link_hover: Hsla,
}

impl PanelColors {
    pub fn from_theme(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            background: theme.background,
            foreground: theme.foreground,
            muted: theme.muted,
            muted_foreground: theme.muted_foreground,
            border: theme.border,
            selection: theme.selection,
            primary: theme.primary,
            secondary_foreground: theme.secondary_foreground,
            success: theme.success,
            danger: theme.danger,
            danger_hover: theme.danger_hover,
            warning: theme.warning,
            list_even: theme.list_even,
            overlay: theme.overlay,
            link: theme.link,
            link_hover: theme.link_hover,
        }
    }
}
