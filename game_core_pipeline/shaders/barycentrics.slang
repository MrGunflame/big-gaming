import types;


// Based on CalcFullBary from
// https://github.com/ConfettiFX/The-Forge/blob/master/Common_3/Renderer/VisibilityBuffer2/Shaders/FSL/VisibilityBufferShadingUtilities.h.fsl
public struct Barycentrics {
    Vec3 m_lambda;
    Vec3 m_ddx;
    Vec3 m_ddy;

    public __init(Vec4 pt0, Vec4 pt1, Vec4 pt2, Vec2 pixel_ndc, Vec2 screen_size) {
        let inv_w = 1.0 / Vec3(pt0.w, pt1.w, pt2.w);

        let ndc0 = pt0.xy * inv_w.x;
        let ndc1 = pt1.xy * inv_w.y;
        let ndc2 = pt2.xy * inv_w.z;

        let inv_det = 1.0 / determinant(Mat2(ndc2 - ndc1, ndc0 - ndc1));
        var m_ddx = Vec3(ndc1.y - ndc2.y, ndc2.y - ndc0.y, ndc0.y - ndc1.y) * inv_det * inv_w;
        var m_ddy = Vec3(ndc2.x - ndc1.x, ndc0.x - ndc2.x, ndc1.x - ndc0.x) * inv_det * inv_w;
        var ddx_sum = dot(m_ddx, Vec3(1.0));
        var ddy_sum = dot(m_ddy, Vec3(1.0));

        let delta_vec = pixel_ndc - ndc0;
        let interp_inv_w = inv_w.x + delta_vec.x * ddx_sum + delta_vec.y * ddy_sum;
        let interp_w = 1.0 / interp_inv_w;

        Vec3 m_lambda;
        m_lambda.x = interp_w * (inv_w.x + delta_vec.x * m_ddx.x + delta_vec.y * m_ddy.x);
        m_lambda.y = interp_w * (delta_vec.x * m_ddx.y + delta_vec.y * m_ddy.y);
        m_lambda.z = interp_w * (delta_vec.x * m_ddx.z + delta_vec.y * m_ddy.z);

        m_ddx *= 2.0 / screen_size.x;
        m_ddy *= 2.0 / screen_size.y;
        ddx_sum *= 2.0 / screen_size.x;
        ddy_sum *= 2.0 / screen_size.y;

        m_ddy *= -1.0;
        ddy_sum *= -1.0;

        let interp_w_ddx = 1.0 / (interp_inv_w + ddx_sum);
        let interp_w_ddy = 1.0 / (interp_inv_w + ddy_sum);

        m_ddx = interp_w_ddx * (m_lambda * interp_inv_w + m_ddx) - m_lambda;
        m_ddy = interp_w_ddy * (m_lambda * interp_inv_w + m_ddy) - m_lambda;

        this.m_lambda = m_lambda;
        this.m_ddx = m_ddx;
        this.m_ddy = m_ddy;
    }

    public Vec3 interpolate(Vec3 a, Vec3 b, Vec3 c) {
        // Effectively a transpose op.
        let x = Vec3(a.x, b.x, c.x);
        let y = Vec3(a.y, b.y, c.y);
        let z = Vec3(a.z, b.z, c.z);
        return Vec3(dot(x, this.m_lambda), dot(y, this.m_lambda), dot(z, this.m_lambda));
    }

    public Vec4 interpolate(Vec4 a, Vec4 b, Vec4 c) {
        let x = Vec3(a.x, b.x, c.x);
        let y = Vec3(a.y, b.y, c.y);
        let z = Vec3(a.z, b.z, c.z);
        let w = Vec3(a.w, b.w, c.w);

        return Vec4(
            dot(x, this.m_lambda),
            dot(y, this.m_lambda),
            dot(z, this.m_lambda),
            dot(w, this.m_lambda),
        );
    }

    public Gradient interpolate_grad(Vec2 a, Vec2 b, Vec2 c) {
        let x = Vec3(a.x, b.x, c.x);
        let y = Vec3(a.y, b.y, c.y);

        Gradient grad;
        grad.value.x = dot(x, this.m_lambda);
        grad.value.y = dot(y, this.m_lambda);

        grad.dx.x = dot(x, this.m_ddx);
        grad.dx.y = dot(y, this.m_ddx);
        grad.dy.x = dot(x, this.m_ddy);
        grad.dy.y = dot(y, this.m_ddy);

        return grad;
    }
}

public struct Gradient {
    public Vec2 value;
    public Vec2 dx;
    public Vec2 dy;
}
