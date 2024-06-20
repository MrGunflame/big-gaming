use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_ui::primitive::Primitive;
use game_ui::reactive::{DocumentId, Node};
use game_ui::render::Text;
use game_ui::style::Style;
use game_ui::UiState;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId, WindowState};
use game_window::WindowManager;

fn main() {
    game_tracing::init();

    let renderer = Renderer::new().unwrap();
    let ui_state = UiState::new(&renderer);
    let pool = TaskPool::new(8);

    let mut wm = WindowManager::new();
    let window_id = wm.windows_mut().spawn(WindowBuilder::new());

    let app = App {
        renderer,
        ui_state,
        pool,
        window_id,
        document_id: None,
    };

    wm.run(app);
}

struct App {
    renderer: Renderer,
    ui_state: UiState,
    pool: TaskPool,
    window_id: WindowId,
    document_id: Option<DocumentId>,
}

impl game_window::App for App {
    fn handle_event(&mut self, ctx: game_window::WindowManagerContext<'_>, event: WindowEvent) {
        match event {
            WindowEvent::WindowCreated(event) => {
                let window = ctx.windows.state(event.window).unwrap();
                self.ui_state
                    .create(RenderTarget::Window(event.window), window.inner_size());
                self.renderer.create(event.window, window);

                let doc = self
                    .ui_state
                    .runtime()
                    .create_document(RenderTarget::Window(event.window))
                    .unwrap();
                self.document_id = Some(doc);

                let rt = self.ui_state.runtime();
                rt.append(
                    doc,
                    None,
                    Node::new(Primitive {
                        style: Style::default(),
                        image: None,
                        text: Some(Text {
                            text: "Hello World!".to_owned(),
                            size: 32.0,
                        }),
                    }),
                );
            }
            WindowEvent::WindowDestroyed(event) => {
                todo!()
            }
            WindowEvent::WindowResized(event) => {
                self.ui_state
                    .resize(RenderTarget::Window(event.window), event.size());
                self.renderer.resize(event.window, event.size());
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: game_window::WindowManagerContext<'_>) {
        self.ui_state.update(&mut Vec::new());
        self.renderer.render(&self.pool);
    }
}
