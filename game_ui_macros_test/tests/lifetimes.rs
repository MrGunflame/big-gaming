use game_ui::component;
use game_ui::reactive::Scope;

#[test]
fn test_named_lifetime() {
    #[component]
    fn Component<'a, 'b>(cx: &Scope, _a: &'a i32, _b: &'b u8) -> Scope {
        cx.clone()
    }
}

#[test]
fn test_unnamed_lifetime() {
    #[component]
    fn Component(cx: &Scope, _a: &i32, _b: &u8) -> Scope {
        cx.clone()
    }
}
