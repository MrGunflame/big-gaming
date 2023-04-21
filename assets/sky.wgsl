// Based on "A Practical Analytic Model for Daylight" or "Preetham model"

const PI: f32 = 3.14159265358979323846264338327950288;

struct Material {
    depolarization_factor: f32,
    mie_coefficient: f32,
    mie_directional_g: f32,
    mie_k_coefficient: vec3<f32>,
    mie_v: f32,
    mie_zenith_length: f32,
    num_molecules: f32,
    primaries: vec3<f32>,
    rayleigh: f32,
    rayleigh_zenith_length: f32,
    refractive_index: f32,
    sun_angular_diameter_degrees: f32,
    sun_intensity_factor: f32,
    sun_intensity_falloff_steepness: f32,
    turbidity: f32,
};

@group(1) @binding(0)
var<uniform> material: Material;

@fragment
fn fragment(
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> @location(0) vec4<f32> {

    // let origin = vec3(0.0, 0.0, 0.0);

    //if world_position.y >= 100.0 {
        //return vec4(0.0, 0.0, 0.0, 0.0);
    //}

    // let f = 10.0 / distance;

    //let r = material.color[0] * f;
    // let g = material.color[1] * f;
    // let b = material.color[2] * f;

    // return vec4(r, g, b, 1.0);

    //return material.color;
    //return vec4(x, y, z, 0.5);
    // return vec4(0.0, 0.0, 0.0, 0.0);
    //return vec4(color[0], color[1], color[2], 0.0);
    // return sin(vec4(world_position[0], world_position[1], world_position[2], 0.0));
    
    // Horizon near factor
//    let f = abs(1.0 / world_position[1]);

    // let h = f / b;

    // return vec4(0.0, 0.0, 0.0, 1.0);

    let color = sky(vec3(1.0, 1.0, 1.0), vec3(0.0, 75.0, -1000.0));
    
    let a = max(color, vec3(0.0, 0.0, 0.0));
    let b = min(a, vec3(1024.0, 1024.0, 1024.0));
    let c = tonemap(b);

    return vec4(c.x, c.y, c.z, 1.0);
}

fn tonemap(col: vec3<f32>) -> vec3<f32> {
    let a = 2.35;
    let b = 2.8826666;
    let c = 789.7459;
    let d = 0.935;

    let z = pow(col, vec3(a, a, a));
    return z / (pow(z, vec3(d, d, d)) * b + vec3(c, c, c));
}

fn total_rayleigh(lambda: vec3<f32>) -> vec3<f32> {
    let refractive_index = material.refractive_index;
    let depolarization_factor = material.depolarization_factor;
    let num_molecules = material.num_molecules;

    let a = (8.0 * pow(PI, 3.0));
    let b = pow((pow(refractive_index, 2.0) - 1.0), 2.0);
    let c = 6.0 * 3.0 * depolarization_factor;
    let d = (3.0 * num_molecules * pow(lambda, vec3(4.0, 4.0, 4.0)) * (6.0 - 7.0 * depolarization_factor));

    return (a * b * c) / d;
}

fn total_mie(lambda: vec3<f32>, k: vec3<f32>, t: f32) -> vec3<f32> {
    let mie_v = material.mie_v;

    let c = 0.2 * t * 10e-18;
    return 0.434 * c * PI * pow((2.0 * PI) / lambda, vec3(mie_v - 2.0, mie_v - 2.0, mie_v - 2.0)) * k;
}

fn rayleigh_phase(cos_theta: f32) -> f32 {
    return (3.0 / (16.0 * PI)) * (1.0 + pow(cos_theta, 2.0));
}

fn henyey_greenstein_phase(cos_theta: f32, g: f32) -> f32 {
    return (1.0 / (4.0 * PI)) * ((1.0 - pow(g, 2.0)) / pow((1.0 - 2.0 * g * cos_theta + pow(g, 2.0)), 1.5));
}

fn sun_intensity(zenith_angle_cos: f32) -> f32 {
    let cutoff_angle = PI / 1.95;

    let sun_intensity_factor = material.sun_intensity_factor;
    let sun_intensity_falloff_steepness = material.sun_intensity_falloff_steepness;

    let a = (cutoff_angle - acos(zenith_angle_cos)) / sun_intensity_falloff_steepness;
    let b = 1.0 - exp(-a);

    return sun_intensity_factor * max(0.0, b);
}

fn sky(dir: vec3<f32>, sun_position: vec3<f32>) -> vec3<f32> {
    let up = vec3(0.0, 1.0, 0.0);

    let primaries = material.primaries;
    let mie_k_coefficient = material.mie_k_coefficient;
    let turbidity = material.turbidity;
    let mie_coefficient = material.mie_coefficient;
    let rayleigh = material.rayleigh;
    let rayleigh_zenith_length = material.rayleigh_zenith_length;
    let mie_zenith_length = material.mie_zenith_length;
    let mie_directional_g = material.mie_directional_g;
    let sun_angular_diameter_degrees = material.sun_angular_diameter_degrees;

    let sunfade = 1.0 - (1.0 - exp(saturate(sun_position.y / 450000.0)));
    let rayleigh_coefficient = rayleigh - (1.0 * (1.0 - sunfade));
    let beta_r = total_rayleigh(primaries) * rayleigh_coefficient;

    let beta_m = total_mie(primaries, mie_k_coefficient, turbidity) * mie_coefficient;

    let zenith_angle = acos(max(dot(up, dir), 0.0));
    let denom = cos(zenith_angle) + 0.15 * pow(93.885 - ((zenith_angle * 180.0) / PI), -1.253);

    let s_r = rayleigh_zenith_length / denom;
    let s_m = mie_zenith_length / denom;

    let fex = exp(-(beta_r * s_r + beta_m * s_m));

    let sun_direction = normalize(sun_position);
    let cos_theta = dot(dir, sun_direction);
    let beta_r_theta = beta_r * rayleigh_phase(cos_theta * 0.5 + 0.5);

    let beta_m_theta = beta_m * henyey_greenstein_phase(cos_theta, mie_directional_g);
    let sun_e = sun_intensity(dot(sun_direction, up));
    
    let lin = pow(sun_e * ((beta_r_theta + beta_m_theta) / (beta_r + beta_m)) * vec3(1.0, 1.0, 1.0) - fex, vec3(1.5, 1.5, 1.5));

    // let a = pow(sun_e * ((beta_r_theta + beta_m_theta) / (beta_r + beta_m)) * fex, 0.5);
    // lin *= lerp(vec3(1.0, 1.0, 1.0), saturate(pow(1.0 - dot(up, sun_direction)), 5.0));
    // lin *= vec3(1.0, 1.0, 1.0);

    let sun_angular_diameter_cos = cos(sun_angular_diameter_degrees);
    let sundisk = smoothstep(sun_angular_diameter_cos, sun_angular_diameter_cos + 0.00002, cos_theta);
    
    var l0:vec3<f32> = 0.1 * fex;
    l0 += sun_e * 19000.0 * fex * sundisk;
    
    return lin + l0;
}

