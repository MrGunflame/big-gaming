mod tests;

use std::sync::Arc;

use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_ui::reactive::DocumentId;
use game_ui::widgets::{Text, Widget};
use game_ui::{UiState, WindowProperties};
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId};
use game_window::WindowManager;
use tests::hello_world::HelloWorld;
use tests::input::input;
use tests::selection::selection;
use tests::svg::TestSvg;
use tests::table::table;

fn main() {
    game_core::logger::init();

    let renderer = Renderer::new().unwrap();
    let ui_state = UiState::new(&renderer);
    let pool = TaskPool::new(8);

    let mut wm = WindowManager::new();
    let window_id = wm.windows_mut().spawn(WindowBuilder::new());
    let cursor = wm.cursor().clone();

    let app = App {
        renderer,
        ui_state,
        pool,
        window_id,
        document_id: None,
        cursor,
    };

    wm.run(app);
}

struct App {
    renderer: Renderer,
    ui_state: UiState,
    pool: TaskPool,
    window_id: WindowId,
    document_id: Option<DocumentId>,
    cursor: Arc<Cursor>,
}

impl game_window::App for App {
    fn handle_event(&mut self, ctx: game_window::WindowManagerContext<'_>, event: WindowEvent) {
        self.ui_state.send_event(&self.cursor, event.clone());

        match event {
            WindowEvent::WindowCreated(event) => {
                let window = ctx.windows.state(event.window).unwrap();
                self.ui_state.create(
                    RenderTarget::Window(event.window),
                    WindowProperties {
                        size: window.inner_size(),
                        scale_factor: window.scale_factor(),
                    },
                );
                self.renderer.create(event.window, window);

                // let doc = self
                //     .ui_state
                //     .runtime()
                //     .create_document(RenderTarget::Window(event.window))
                //     .unwrap();
                // self.document_id = Some(doc);

                // let rt = self.ui_state.runtime();
                // let ctx = rt.root_context(doc);

                self.ui_state
                    .runtime()
                    .mount(RenderTarget::Window(event.window), TestSvg);

                // hello_world(ctx.clone());
                // selection(ctx);
                // svg(ctx);
                // input(ctx);
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
        self.ui_state.update();
        self.renderer.render(&self.pool);
    }
}
