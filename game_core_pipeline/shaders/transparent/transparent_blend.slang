import types;
import shared;

static const u32 NUM_THREADS = 64;

static const u32 OIT_LAYERS = 8;

[[vk::push_constant]] vector<u32, 2> output_size;

[[vk::binding(0, 0)]] StructuredBuffer<Element> elements;
[[vk::binding(1, 0)]] Texture2D<u32> heads;
[[vk::binding(2, 0)]] RWTexture2D<Vec4> output;

[shader("compute")]
[numthreads(NUM_THREADS, 1, 1)]
void main(
    u32 tid: SV_GroupThreadID,
    u32 gid: SV_GroupID,
) {
    // TODO: Group into tiles instead of going row by row
    // for better cache locality.
    let id = gid * NUM_THREADS + tid;
    let x = id % output_size.x;
    let y = id / output_size.x;

    if (x >= output_size.x || y >= output_size.y) {
        return;
    }

    let coords = vector<u32, 2>(x, y);
    
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

        // if (color.a > 0.5) {
            let premult = Vec4(layers[i].color.rgb * layers[i].color.a, layers[i].color.a);
            color.rgb = (1.0 - premult.a) * color.rgb + premult.rgb;
            color.a = color.a + premult.a - color.a * premult.a;

            // color.rgb = color.rgb + premult.rgb;
            // color.rgb = color.rgb - premult.rgb;
            // color.rgb = color.rgb;

            let nan = 0.0 / 0.0;

    // if (color.r == nan || color.g == nan || color.b == nan) {
    //     color = Vec4(1.0, 1.0, 1.0, 1.0);
    // }

            // color = premult;
        // }
    }

    // let nan = 0.0 / 0.0;

    // if (color.r == nan || color.g == nan || color.b == nan) {
    //     color = Vec4(1.0, 1.0, 1.0, 1.0);
    // }

    output[coords] = color;
}
