import types;
import pbr;

public struct FragParams {
    public vector<f32, 3> world_position;
}

public struct DirectionalLight {
    public vector<f32, 3> direction;
    public vector<f32, 3> color;
    public f32 intensity;

    public func eval() -> pbr::Light {
        pbr::Light light;
        light.color = this.color * this.intensity;
        light.attenuation = 1.0;
        light.direction = normalize(-this.direction);
        return light;
    }
}

public struct PointLight {
    public Vec3 position;
    public Vec3 color;
    public f32 intensity;
    public f32 radius;

    public func eval(frag: FragParams) -> pbr::Light {
        let distance = length(this.position - frag.world_position);
        let pos_to_light = this.position - frag.world_position;
        let attenuation = get_distance_attenuation(dot(pos_to_light, pos_to_light), (1.0 / this.radius) * (1.0 / this.radius));

        let light_dir = normalize(this.position - frag.world_position);

        pbr::Light light;
        light.color = this.color * this.intensity;
        light.attenuation = attenuation;
        light.direction = light_dir;
        return light;
    }
}

public struct SpotLight {
    public Vec3 position;
    public Vec3 direction;
    public Vec3 color;
    public f32 intensity;
    public f32 radius;
    public f32 inner_cutoff;
    public f32 outer_cutoff;

    public func eval(frag: FragParams) -> pbr::Light {
        let pos_to_light = this.position - frag.world_position;
        let attenuation = get_distance_attenuation(dot(pos_to_light, pos_to_light), (1.0 / this.radius) * (1.0 / this.radius));

        let light_dir = normalize(this.position - frag.world_position);

        // Falloff
        // TODO: cosine can be precomputed on CPU side.
        let cos_outer = cos(this.outer_cutoff);
        let cos_inner = cos(this.inner_cutoff);

        let theta = dot(light_dir, -this.direction);

        let epsilon = cos_inner - cos_outer;
        let intensity = clamp((theta - cos_outer) / epsilon, 0.0, 1.0);

        pbr::Light light;
        light.color = this.color * this.intensity;
        light.attenuation = attenuation;
        light.direction = light_dir;
        return light;
    }
}

func get_distance_attenuation(distance_square: f32, inv_range_squared: f32) -> f32 {
    let factor = distance_square * inv_range_squared;
    let smooth_factor = saturate(1.0 - factor * factor);
    let attenuation = smooth_factor * smooth_factor;
    return attenuation / max(distance_square, 0.0001);
}
