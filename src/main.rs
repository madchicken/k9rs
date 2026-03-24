mod app;
mod k8s;
mod model;
mod ui;

use clap::Parser;
use gpui::*;
use tracing_subscriber::EnvFilter;

actions!(k9rs, [Quit]);

#[derive(Parser, Debug)]
#[command(name = "k9rs", version, about = "Kubernetes CLI Manager")]
pub struct Args {
    /// Kubernetes namespace to use
    #[arg(short, long, default_value = "default")]
    namespace: String,

    /// Kubernetes context to use (defaults to current-context from kubeconfig)
    #[arg(short, long)]
    context: Option<String>,

    /// Resource to display on startup
    #[arg(short, long, default_value = "pods")]
    resource: String,
}

fn main() {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Launch the GPUI application
    let app = Application::new();
    app.run(move |cx: &mut App| {
        // Initialize gpui-component (themes, input, etc.)
        gpui_component::init(cx);

        // Apply Catppuccin Mocha theme to gpui-component
        {
            let theme = gpui_component::Theme::global_mut(cx);
            theme.mode = gpui_component::theme::ThemeMode::Dark;
            theme.mono_font_family = "Monaco".into();
            theme.mono_font_size = px(14.);

            // Catppuccin Mocha colors
            let base = hsla(232. / 360., 0.23, 0.18, 1.0); // #1e1e2e
            let surface0 = hsla(230. / 360., 0.14, 0.24, 1.0); // #313244
            let surface1 = hsla(231. / 360., 0.16, 0.34, 1.0); // #45475a
            let surface2 = hsla(232. / 360., 0.12, 0.39, 1.0); // #585b70
            let overlay0 = hsla(228. / 360., 0.11, 0.48, 1.0); // #6c7086
            let text = hsla(226. / 360., 0.64, 0.88, 1.0); // #cdd6f4
            let subtext = hsla(227. / 360., 0.35, 0.80, 1.0); // #bac2de
            let blue = hsla(217. / 360., 0.92, 0.76, 1.0); // #89b4fa

            theme.colors.background = base;
            theme.colors.foreground = text;
            theme.colors.border = surface1;
            theme.colors.input = surface1;
            theme.colors.muted = surface0;
            theme.colors.muted_foreground = overlay0;
            theme.colors.selection = surface2;
            theme.colors.accent = surface2;
            theme.colors.primary = blue;
            theme.colors.primary_foreground = base;
            theme.colors.caret = blue;
            theme.colors.ring = blue;
            theme.colors.scrollbar_thumb = surface1;
            theme.colors.scrollbar = base;

            // Tab colors
            theme.colors.tab = surface0;
            theme.colors.tab_active = surface1;
            theme.colors.tab_active_foreground = blue;
            theme.colors.tab_bar = surface0;
            theme.colors.tab_foreground = subtext;

            // Table colors — table_active overlay is absolute-positioned on top of text,
            // so it must be semi-transparent for text to remain visible
            theme.colors.table = base;
            theme.colors.table_active = hsla(217. / 360., 0.92, 0.76, 0.15); // blue tint, semi-transparent
            theme.colors.table_active_border = hsla(217. / 360., 0.92, 0.76, 0.4); // subtle blue border
            theme.colors.table_even = hsla(233. / 360., 0.20, 0.20, 1.0); // #24243a
            theme.colors.table_head = surface1;
            theme.colors.table_head_foreground = blue;
            theme.colors.table_hover = hsla(232. / 360., 0.20, 0.28, 0.5); // semi-transparent hover
            theme.colors.table_row_border = hsla(0., 0., 0., 0.0); // transparent
            theme.colors.secondary_foreground = text;

            // Editor-specific colors (highlight theme)
            let ht = std::sync::Arc::make_mut(&mut theme.highlight_theme);
            ht.style.editor_background = Some(base);
            ht.style.editor_foreground = Some(text);
            ht.style.editor_active_line = Some(surface0);
            ht.style.editor_line_number = Some(overlay0);
            ht.style.editor_active_line_number = Some(subtext);
        }

        // Register global actions
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

        // Key bindings
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            // App navigation — use "app" context which is on the root div
            KeyBinding::new("up", app::MoveUp, Some("app")),
            KeyBinding::new("k", app::MoveUp, Some("app")),
            KeyBinding::new("down", app::MoveDown, Some("app")),
            KeyBinding::new("j", app::MoveDown, Some("app")),
            KeyBinding::new("enter", app::Enter, Some("app")),
            KeyBinding::new("escape", app::GoBack, Some("app")),
            KeyBinding::new(":", app::ActivateCommand, Some("app")),
            KeyBinding::new("backspace", app::Backspace, Some("app")),
            KeyBinding::new("/", app::ActivateFilter, Some("app")),
            KeyBinding::new("tab", app::ToggleSidebar, Some("app")),
            // Detail tab switching
            KeyBinding::new("1", app::DetailTab1, Some("app")),
            KeyBinding::new("2", app::DetailTab2, Some("app")),
            KeyBinding::new("3", app::DetailTab3, Some("app")),
            KeyBinding::new("4", app::DetailTab4, Some("app")),
            // Actions
            KeyBinding::new("r", app::RestartResource, Some("app")),
            KeyBinding::new("f", app::OpenPortForward, Some("app")),
            KeyBinding::new("d", app::StopPortForward, Some("app")),
            KeyBinding::new("ctrl-s", app::ApplyYaml, Some("app")),
            KeyBinding::new("ctrl-n", app::ToggleNamespacePicker, Some("app")),
        ]);

        // Open the main window
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.0), px(800.0)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("k9rs - Kubernetes CLI Manager".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let namespace = args.namespace.clone();
        let context = args.context.clone();
        let resource = args.resource.clone();

        // Create the AppView first, keep a handle to focus it later
        let app_entity: std::cell::RefCell<Option<Entity<app::AppView>>> =
            std::cell::RefCell::new(None);
        let app_entity_ref = &app_entity;

        let window_handle = cx
            .open_window(window_options, |window, cx| {
                let view = cx.new(|cx| {
                    app::AppView::new(cx, window, &namespace, context.as_deref(), &resource)
                });
                *app_entity_ref.borrow_mut() = Some(view.clone());
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            })
            .unwrap();

        // Activate the app and window, focus the AppView
        cx.activate(true);
        let app_view = app_entity.borrow().clone().unwrap();
        window_handle
            .update(cx, |_root, window, cx| {
                window.activate_window();
                // Focus the table so keyboard navigation works immediately
                let app = app_view.read(cx);
                let table_handle = app.table_state.read(cx).focus_handle(cx);
                table_handle.focus(window);
            })
            .ok();
    });
}
