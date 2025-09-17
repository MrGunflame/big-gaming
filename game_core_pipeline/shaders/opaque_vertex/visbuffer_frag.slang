import types;

struct Input {
    vector<f32, 4> clip_position: SV_Position;
    [[vk::location(0)]] u32 instance_id;
    [[vk::location(1)]] u32 vertex_id;
}

struct Output {
    [[vk::location(0)]] vector<u32, 2> ids;
}

[shader("fragment")]
func main(input: Input) -> Output {
    Output output;
    output.ids.x = input.instance_id;
    output.ids.y = input.vertex_id;
    return output;
}
