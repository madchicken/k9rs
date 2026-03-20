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
        // Register global actions
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

        // Key bindings
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            // App navigation
            KeyBinding::new("up", app::MoveUp, Some("app")),
            KeyBinding::new("k", app::MoveUp, Some("app")),
            KeyBinding::new("down", app::MoveDown, Some("app")),
            KeyBinding::new("j", app::MoveDown, Some("app")),
            KeyBinding::new("enter", app::Enter, Some("app")),
            KeyBinding::new("escape", app::GoBack, Some("app")),
            KeyBinding::new(":", app::ActivateCommand, Some("app")),
            KeyBinding::new("backspace", app::Backspace, Some("app")),
            // Filter
            KeyBinding::new("/", app::ActivateFilter, Some("app")),
            // Panel navigation
            KeyBinding::new("tab", app::ToggleSidebar, Some("app")),
            // Detail tab switching
            KeyBinding::new("1", app::DetailTab1, Some("app")),
            KeyBinding::new("2", app::DetailTab2, Some("app")),
            KeyBinding::new("3", app::DetailTab3, Some("app")),
            KeyBinding::new("4", app::DetailTab4, Some("app")),
            // Namespace picker toggle
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

        let window_handle = cx
            .open_window(window_options, |window, cx| {
                cx.new(|cx| app::AppView::new(cx, window, &namespace, context.as_deref(), &resource))
            })
            .unwrap();

        // Activate the app and window so it comes to front and gets focus
        cx.activate(true);
        window_handle
            .update(cx, |_view, window, cx| {
                window.activate_window();
                cx.focus_self(window);
            })
            .ok();
    });
}
