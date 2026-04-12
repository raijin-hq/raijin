use inazuma::Oklch;

/// APCA (Accessible Perceptual Contrast Algorithm) constants
/// Based on APCA 0.0.98G-4g W3 compatible constants
/// https://github.com/Myndex/apca-w3
struct APCAConstants {
    main_trc: f32,
    s_rco: f32,
    s_gco: f32,
    s_bco: f32,
    norm_bg: f32,
    norm_txt: f32,
    rev_txt: f32,
    rev_bg: f32,
    blk_thrs: f32,
    blk_clmp: f32,
    scale_bow: f32,
    scale_wob: f32,
    lo_bow_offset: f32,
    lo_wob_offset: f32,
    delta_y_min: f32,
    lo_clip: f32,
}

impl Default for APCAConstants {
    fn default() -> Self {
        Self {
            main_trc: 2.4,
            s_rco: 0.2126729,
            s_gco: 0.7151522,
            s_bco: 0.0721750,
            norm_bg: 0.56,
            norm_txt: 0.57,
            rev_txt: 0.62,
            rev_bg: 0.65,
            blk_thrs: 0.022,
            blk_clmp: 1.414,
            scale_bow: 1.14,
            scale_wob: 1.14,
            lo_bow_offset: 0.027,
            lo_wob_offset: 0.027,
            delta_y_min: 0.0005,
            lo_clip: 0.1,
        }
    }
}

/// Calculates the perceptual lightness contrast using APCA.
fn apca_contrast(text_color: Oklch, background_color: Oklch) -> f32 {
    let constants = APCAConstants::default();

    let text_y = srgb_to_y(text_color, &constants);
    let bg_y = srgb_to_y(background_color, &constants);

    let text_y_clamped = if text_y > constants.blk_thrs {
        text_y
    } else {
        text_y + (constants.blk_thrs - text_y).powf(constants.blk_clmp)
    };

    let bg_y_clamped = if bg_y > constants.blk_thrs {
        bg_y
    } else {
        bg_y + (constants.blk_thrs - bg_y).powf(constants.blk_clmp)
    };

    if (bg_y_clamped - text_y_clamped).abs() < constants.delta_y_min {
        return 0.0;
    }

    let sapc;
    let output_contrast;

    if bg_y_clamped > text_y_clamped {
        sapc = (bg_y_clamped.powf(constants.norm_bg) - text_y_clamped.powf(constants.norm_txt))
            * constants.scale_bow;

        output_contrast = if sapc < constants.lo_clip {
            0.0
        } else {
            sapc - constants.lo_bow_offset
        };
    } else {
        sapc = (bg_y_clamped.powf(constants.rev_bg) - text_y_clamped.powf(constants.rev_txt))
            * constants.scale_wob;

        output_contrast = if sapc > -constants.lo_clip {
            0.0
        } else {
            sapc + constants.lo_wob_offset
        };
    }

    output_contrast * 100.0
}

fn srgb_to_y(color: Oklch, constants: &APCAConstants) -> f32 {
    let rgba = color.to_rgb();

    let r_linear = (rgba.r).powf(constants.main_trc);
    let g_linear = (rgba.g).powf(constants.main_trc);
    let b_linear = (rgba.b).powf(constants.main_trc);

    constants.s_rco * r_linear + constants.s_gco * g_linear + constants.s_bco * b_linear
}

/// Adjusts the foreground color to meet the minimum APCA contrast against the background.
pub fn ensure_minimum_contrast(
    foreground: Oklch,
    background: Oklch,
    minimum_apca_contrast: f32,
) -> Oklch {
    if minimum_apca_contrast <= 0.0 {
        return foreground;
    }

    let current_contrast = apca_contrast(foreground, background).abs();

    if current_contrast >= minimum_apca_contrast {
        return foreground;
    }

    let adjusted = adjust_lightness_for_contrast(foreground, background, minimum_apca_contrast);

    let adjusted_contrast = apca_contrast(adjusted, background).abs();
    if adjusted_contrast >= minimum_apca_contrast {
        return adjusted;
    }

    let desaturated =
        adjust_lightness_and_saturation_for_contrast(foreground, background, minimum_apca_contrast);

    let desaturated_contrast = apca_contrast(desaturated, background).abs();
    if desaturated_contrast >= minimum_apca_contrast {
        return desaturated;
    }

    let black = Oklch {
        l: 0.0,
        c: 0.0,
        h: 0.0,
        a: foreground.a,
    };

    let white = Oklch {
        l: 1.0,
        c: 0.0,
        h: 0.0,
        a: foreground.a,
    };

    let black_contrast = apca_contrast(black, background).abs();
    let white_contrast = apca_contrast(white, background).abs();

    if white_contrast > black_contrast {
        white
    } else {
        black
    }
}

fn adjust_lightness_for_contrast(
    foreground: Oklch,
    background: Oklch,
    minimum_apca_contrast: f32,
) -> Oklch {
    let bg_luminance = srgb_to_y(background, &APCAConstants::default());
    let should_go_darker = bg_luminance > 0.5;

    let mut low = if should_go_darker { 0.0 } else { foreground.l };
    let mut high = if should_go_darker { foreground.l } else { 1.0 };
    let mut best_l = foreground.l;

    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        let test_color = Oklch {
            l: mid,
            c: foreground.c,
            h: foreground.h,
            a: foreground.a,
        };

        let contrast = apca_contrast(test_color, background).abs();

        if contrast >= minimum_apca_contrast {
            best_l = mid;
            if should_go_darker {
                low = mid;
            } else {
                high = mid;
            }
        } else if should_go_darker {
            high = mid;
        } else {
            low = mid;
        }

        if (contrast - minimum_apca_contrast).abs() < 1.0 {
            best_l = mid;
            break;
        }
    }

    Oklch {
        l: best_l,
        c: foreground.c,
        h: foreground.h,
        a: foreground.a,
    }
}

fn adjust_lightness_and_saturation_for_contrast(
    foreground: Oklch,
    background: Oklch,
    minimum_apca_contrast: f32,
) -> Oklch {
    let saturation_steps = [1.0, 0.8, 0.6, 0.4, 0.2, 0.0];

    for &sat_multiplier in &saturation_steps {
        let test_color = Oklch {
            l: foreground.l,
            c: foreground.c * sat_multiplier,
            h: foreground.h,
            a: foreground.a,
        };

        let adjusted = adjust_lightness_for_contrast(test_color, background, minimum_apca_contrast);
        let contrast = apca_contrast(adjusted, background).abs();

        if contrast >= minimum_apca_contrast {
            return adjusted;
        }
    }

    Oklch {
        l: foreground.l,
        c: 0.0,
        h: foreground.h,
        a: foreground.a,
    }
}
