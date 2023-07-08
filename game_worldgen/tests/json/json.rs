use game_worldgen::data::json;

#[test]
fn load_json() {
    let input = include_bytes!("input.json");
    let cells = json::from_slice(input).unwrap();
}
