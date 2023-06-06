use core::arch::x86_64::{
    __m128, _mm_add_ps, _mm_cvtss_f32, _mm_div_ps, _mm_mul_ps, _mm_set1_ps, _mm_set_ps,
};
use core::mem::transmute;

use image::{GenericImage, GenericImageView, ImageBuffer, Rgba};

pub(super) unsafe fn overlay(
    bottom: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    top: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: u32,
    y: u32,
) {
    let start_x = x;
    let start_y = y;
    let end_x = x + top.width();
    let end_y = y + top.height();

    for x in start_x..end_x {
        for y in start_y..end_y {
            let top_x = x - start_x;
            let top_y = y - start_y;

            if cfg!(debug_assertions) {
                bottom.get_pixel(x, y);
                top.get_pixel(top_x, top_y);
            }

            let bottom_px = unsafe { bottom.unsafe_get_pixel(x, y) };
            let top_px = unsafe { top.unsafe_get_pixel(top_x, top_y) };

            let pixel = Rgba(unsafe { blend(bottom_px.0, top_px.0) });

            unsafe { bottom.unsafe_put_pixel(x, y, pixel) };
        }
    }
}

unsafe fn blend(a: [u8; 4], b: [u8; 4]) -> [u8; 4] {
    if b[3] == 0 {
        return a;
    }

    if b[3] == 255 {
        return b;
    }

    unsafe {
        let byte_vec = _mm_set1_ps(255.0);

        let mut bg = _mm_set_ps(a[0] as f32, a[1] as f32, a[2] as f32, a[3] as f32);
        let mut fg = _mm_set_ps(b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32);

        bg = _mm_div_ps(bg, byte_vec);
        fg = _mm_div_ps(fg, byte_vec);

        let bg_a = _mm_cvtss_f32(bg);
        let fg_a = _mm_cvtss_f32(fg);

        let final_alpha = bg_a + fg_a - bg_a * fg_a;

        let bg_a_pre = {
            let bg_a = _mm_set1_ps(bg_a);
            _mm_mul_ps(bg, bg_a)
        };

        let fg_a_pre = {
            let fg_a = _mm_set1_ps(fg_a);
            _mm_mul_ps(fg, fg_a)
        };

        let out_a = {
            let a = _mm_set1_ps(1.0 - fg_a);
            let p1 = _mm_mul_ps(bg_a_pre, a);
            _mm_add_ps(fg_a_pre, p1)
        };

        let out = {
            let a = _mm_set1_ps(final_alpha);
            _mm_div_ps(out_a, a)
        };

        let res = {
            let arr = transmute::<__m128, [f32; 4]>(out);

            let r = arr[3];
            let g = arr[2];
            let b = arr[1];

            let v = _mm_set_ps(final_alpha, b, g, r);
            let v = _mm_mul_ps(v, byte_vec);

            transmute::<__m128, [f32; 4]>(v)
        };

        [res[0] as u8, res[1] as u8, res[2] as u8, res[3] as u8]
    }
}
