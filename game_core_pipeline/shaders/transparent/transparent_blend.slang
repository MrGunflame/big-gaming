import types;
import shared;

static const u32 NUM_THREADS = 64;

static const u32 OIT_LAYERS = 8;

[[vk::push_constant]] vector<u32, 2> output_size;

[[vk::binding(0, 0)]] StructuredBuffer<Element> elements;
[[vk::binding(1, 0)]] Texture2D<u32> heads;
[[vk::binding(2, 0)]] RWTexture2D<Vec4> output;

static const u32 TILE_SIZE = 16;

[shader("compute")]
[numthreads(TILE_SIZE, TILE_SIZE, 1)]
void main(
    vector<u32, 2> tid: SV_GroupThreadID,
    vector<u32, 2> gid: SV_GroupID,
) {
    let coords = gid * TILE_SIZE + tid;
    if (coords.x >= output_size.x || coords.y >= output_size.y) {
        return;
    }
    
    Element layers[OIT_LAYERS];
    var layer_count = 0;

    var next = heads[coords];
    while (next != 0 && layer_count < OIT_LAYERS) {
        let elem = elements[next];

        layers[layer_count] = elem;
        next = elem.next;
        layer_count++;
    }

    // Sort layers by depth (Insertion sort)
    for (int i = 1; i < layer_count; i++) {
        let elem = layers[i];
        var j = i;

        while (j > 0 && layers[j - 1].depth > elem.depth) {
            layers[j] = layers[j - 1];
            j--;
        }

        layers[j] = elem;
    }

    var color = output[coords];

    for (int i = 0; i < layer_count; i++) {
        let top = Vec4(layers[i].color);
        let premult = Vec4(top.rgb * top.a, top.a);
        color.rgb = (1.0 - premult.a) * color.rgb + premult.rgb;
        color.a = color.a + premult.a - color.a * premult.a;
    }

    output[coords] = color;
}
