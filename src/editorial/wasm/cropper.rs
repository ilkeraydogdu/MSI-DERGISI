//! Interactive `1200x675` (16:9) HTML5 Canvas image cropper with pan, zoom (`1..3`), CORS detection, and `0.9` JPEG Blob export.

use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

use crate::editorial::wasm::state::{is_valid_image_url, with_state, with_state_mut};

pub const CROP_WIDTH: f64 = 1200.0;
pub const CROP_HEIGHT: f64 = 675.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropGeometry {
    pub draw_x: f64,
    pub draw_y: f64,
    pub disp_w: f64,
    pub disp_h: f64,
    pub clamped_pan_x: f64,
    pub clamped_pan_y: f64,
}

/// Computes exact cover-fit + zoom + clamped pan coordinates in `1200x675` canvas space.
pub fn compute_crop_geometry(
    img_w: f64,
    img_h: f64,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
) -> CropGeometry {
    let safe_w = img_w.max(1.0);
    let safe_h = img_h.max(1.0);
    let z = zoom.clamp(1.0, 3.0);

    let base_scale = (CROP_WIDTH / safe_w).max(CROP_HEIGHT / safe_h);
    let scale = base_scale * z;
    let disp_w = safe_w * scale;
    let disp_h = safe_h * scale;

    let max_pan_x = ((disp_w - CROP_WIDTH) / 2.0).max(0.0);
    let max_pan_y = ((disp_h - CROP_HEIGHT) / 2.0).max(0.0);

    let clamped_pan_x = pan_x.clamp(-max_pan_x, max_pan_x);
    let clamped_pan_y = pan_y.clamp(-max_pan_y, max_pan_y);

    let draw_x = (CROP_WIDTH - disp_w) / 2.0 + clamped_pan_x;
    let draw_y = (CROP_HEIGHT - disp_h) / 2.0 + clamped_pan_y;

    CropGeometry {
        draw_x,
        draw_y,
        disp_w,
        disp_h,
        clamped_pan_x,
        clamped_pan_y,
    }
}

/// Pure-Rust base64 decoder for converting `canvas.toDataURL("image/jpeg", 0.9)` into bytes.
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity((input.len() * 3) / 4);
    let mut buf: u32 = 0;
    let mut bits: u8 = 0;

    for b in input.bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'=' {
            break;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

fn find_cropper_canvas() -> Option<HtmlCanvasElement> {
    let doc = web_sys::window()?.document()?;
    let el = doc.get_element_by_id("hf-cropper-canvas")?;
    el.dyn_into::<HtmlCanvasElement>().ok()
}

/// Synchronizes or starts loading the Step 2 cropper image and draws onto `#hf-cropper-canvas`.
pub fn sync_cropper_canvas() {
    let (step, img_url, already_loaded, has_error) = with_state(|s| {
        let u = s
            .editing_article
            .effective_image_url()
            .unwrap_or_default()
            .to_string();
        let loaded = s.crop_loaded_src == u && s.crop_image_el.is_some();
        (s.editor_step, u, loaded, s.editor_image_error)
    });

    if step != 2 || !is_valid_image_url(Some(&img_url)) || has_error {
        return;
    }

    if already_loaded {
        redraw_cropper_canvas();
        return;
    }

    let Ok(img) = HtmlImageElement::new() else {
        return;
    };
    img.set_cross_origin(Some("anonymous"));

    let rc_img = Rc::new(img);
    let url_for_load = img_url.clone();
    let img_for_load = Rc::clone(&rc_img);

    let onload = Closure::wrap(Box::new(move || {
        let still_matches = with_state(|s| {
            s.editing_article
                .effective_image_url()
                .map(|u| u == url_for_load)
                .unwrap_or(false)
        });
        if !still_matches {
            return;
        }
        with_state_mut(|s| {
            s.crop_loaded_src = url_for_load.clone();
            s.crop_image_el = Some(Rc::clone(&img_for_load));
            s.crop_zoom = 1.0;
            s.crop_pan_x = 0.0;
            s.crop_pan_y = 0.0;
            s.editor_image_error = false;
        });
        redraw_cropper_canvas();
    }) as Box<dyn FnMut()>);

    let url_for_err = img_url.clone();
    let onerror = Closure::wrap(Box::new(move || {
        let still_matches = with_state(|s| {
            s.editing_article
                .effective_image_url()
                .map(|u| u == url_for_err)
                .unwrap_or(false)
        });
        if !still_matches {
            return;
        }
        with_state_mut(|s| {
            s.crop_image_el = None;
            s.crop_loaded_src.clear();
            s.editor_image_error = true;
        });
        crate::editorial::wasm::render::refresh_right_panel();
    }) as Box<dyn FnMut()>);

    rc_img.set_onload(Some(onload.as_ref().unchecked_ref()));
    rc_img.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onload.forget();
    onerror.forget();

    rc_img.set_src(&img_url);
}

/// Redraws the current image onto `#hf-cropper-canvas` at `1200x675` using current zoom and pan.
pub fn redraw_cropper_canvas() {
    let Some(canvas) = find_cropper_canvas() else {
        return;
    };
    let (maybe_img, zoom, pan_x, pan_y) = with_state(|s| {
        (
            s.crop_image_el.clone(),
            s.crop_zoom,
            s.crop_pan_x,
            s.crop_pan_y,
        )
    });
    let Some(img) = maybe_img else {
        return;
    };

    let nat_w = img.natural_width() as f64;
    let nat_h = img.natural_height() as f64;
    if nat_w <= 0.0 || nat_h <= 0.0 {
        return;
    }

    canvas.set_width(CROP_WIDTH as u32);
    canvas.set_height(CROP_HEIGHT as u32);

    let Ok(Some(ctx_obj)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(ctx) = ctx_obj.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };

    let geom = compute_crop_geometry(nat_w, nat_h, zoom, pan_x, pan_y);
    with_state_mut(|s| {
        s.crop_pan_x = geom.clamped_pan_x;
        s.crop_pan_y = geom.clamped_pan_y;
    });

    ctx.set_fill_style_str("#18181b");
    ctx.fill_rect(0.0, 0.0, CROP_WIDTH, CROP_HEIGHT);
    let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
        &img,
        geom.draw_x,
        geom.draw_y,
        geom.disp_w,
        geom.disp_h,
    );
}

/// Exports the active `1200x675` cropper canvas to a `0.9` quality `image/jpeg` `Blob`.
pub fn export_cropped_jpeg_blob() -> Result<Option<Blob>, String> {
    let has_valid_img = with_state(|s| {
        is_valid_image_url(s.editing_article.effective_image_url()) && !s.editor_image_error
    });
    if !has_valid_img {
        return Ok(None);
    }

    redraw_cropper_canvas();

    let Some(canvas) = find_cropper_canvas() else {
        return Ok(None);
    };

    let has_loaded_el = with_state(|s| s.crop_image_el.is_some());
    if !has_loaded_el {
        return Ok(None);
    }

    let data_url = canvas
        .to_data_url_with_type_and_encoder_options("image/jpeg", &JsValue::from_f64(0.9))
        .map_err(|_| {
            with_state_mut(|s| s.editor_image_error = true);
            crate::editorial::wasm::render::refresh_right_panel();
            "Görsel kırpılırken CORS engeli oluştu. Sunucuya çekip tekrar deneyin.".to_string()
        })?;

    let (_, b64_part) = data_url
        .split_once(";base64,")
        .ok_or_else(|| "Geçersiz JPEG verisi.".to_string())?;
    let bytes =
        decode_base64(b64_part).ok_or_else(|| "JPEG base64 çözümlenemedi.".to_string())?;

    let u8_arr = js_sys::Uint8Array::from(bytes.as_slice());
    let parts = js_sys::Array::new();
    parts.push(&u8_arr.buffer());

    let bag = BlobPropertyBag::new();
    bag.set_type("image/jpeg");

    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
        .map_err(|_| "Görsel Blob oluşturulamadı.".to_string())?;
    Ok(Some(blob))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_exact_1200x675_cover_and_pan_bounds() {
        // 2400x1350 (exact 16:9) at zoom 1.0 -> exact 1200x675, zero pan room
        let g1 = compute_crop_geometry(2400.0, 1350.0, 1.0, 150.0, -90.0);
        assert!((g1.disp_w - 1200.0).abs() < 1e-6);
        assert!((g1.disp_h - 675.0).abs() < 1e-6);
        assert!((g1.clamped_pan_x).abs() < 1e-6);
        assert!((g1.clamped_pan_y).abs() < 1e-6);

        // Zoom 2.0 -> 2400x1350 displayed, max pan X = 600, max pan Y = 337.5
        let g2 = compute_crop_geometry(2400.0, 1350.0, 2.0, 999.0, -500.0);
        assert!((g2.disp_w - 2400.0).abs() < 1e-6);
        assert!((g2.clamped_pan_x - 600.0).abs() < 1e-6);
        assert!((g2.clamped_pan_y - (-337.5)).abs() < 1e-6);
    }

    #[test]
    fn decodes_base64_accurately() {
        let decoded = decode_base64("TVNJ").expect("valid base64");
        assert_eq!(decoded, b"MSI");
    }
}
