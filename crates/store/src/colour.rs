//! Colour management of picture files (S27, DECISIONS D53): every pixel inside the app is sRGB.
//! A picture coming in is read in the colour space its embedded ICC profile names (PNG `iCCP`,
//! JPEG APP2) and converted into sRGB; an exported file declares sRGB with an embedded profile.
//! moxcms does the profiles and the conversion; this module only decides what a profile means.

use moxcms::{
    ColorProfile, DataColorSpace, Layout, RenderingIntent, TransformExecutor, TransformOptions,
};

/// The sRGB profile every exported file carries: moxcms's sRGB (`ColorProfile::new_srgb`),
/// encoded once and kept, so every export carries the same 612 bytes (encoding stamps the time).
pub(crate) const SRGB_PROFILE: &[u8] = include_bytes!("../assets/srgb.icc");

/// What a picture's colour information meant on the way in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interpretation {
    /// No profile: the pixels are taken as sRGB (the web's and Android's rule).
    Untagged,
    /// An RGB profile under which every probe colour maps to itself: sRGB already, kept as is.
    Srgb,
    /// Converted from the profile's colour space into sRGB.
    Converted,
    /// A profile that does not parse or does not describe RGB (grey, CMYK): taken as sRGB.
    Ignored,
}

/// Pixels converted per call, so a large picture needs no second full-size buffer.
const CHUNK: usize = 4 * 65_536;

/// Turns straight (not premultiplied) 8-bit RGBA `pixels`, encoded as `profile` describes, into
/// sRGB in place; alpha stays. Colours outside sRGB are mapped by the profile's perceptual
/// intent where it has one (a LUT profile's own gamut mapping) and clipped per channel otherwise
/// (matrix profiles such as Display P3 or Adobe RGB define no perceptual mapping).
pub(crate) fn into_srgb(profile: Option<&[u8]>, pixels: &mut [u8]) -> Interpretation {
    let Some(bytes) = profile else {
        return Interpretation::Untagged;
    };
    let source = match ColorProfile::new_from_slice(bytes) {
        Ok(source) if source.color_space == DataColorSpace::Rgb => source,
        _ => return Interpretation::Ignored,
    };
    let options = TransformOptions {
        rendering_intent: RenderingIntent::Perceptual,
        ..TransformOptions::default()
    };
    let srgb = ColorProfile::new_srgb();
    let Ok(transform) = source.create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgba, options)
    else {
        return Interpretation::Ignored;
    };
    if !pixels.len().is_multiple_of(4) {
        return Interpretation::Ignored;
    }
    if maps_to_itself(&*transform) {
        return Interpretation::Srgb;
    }
    let mut converted = vec![0; CHUNK.min(pixels.len())];
    for chunk in pixels.chunks_mut(CHUNK) {
        let converted = &mut converted[..chunk.len()];
        // Whole pixels in and out (checked above), so moxcms has nothing to refuse.
        if transform.transform(chunk, converted).is_err() {
            return Interpretation::Ignored;
        }
        chunk.copy_from_slice(converted);
    }
    Interpretation::Converted
}

/// Whether `transform` leaves every level of each primary, of grey and of two mixtures as it
/// was: then the profile is sRGB (or as near as 8 bits tell) and the pixels stay bit for bit,
/// so a file the app exported comes back in exactly.
fn maps_to_itself(transform: &(dyn TransformExecutor<u8> + Send + Sync)) -> bool {
    let probe: Vec<u8> = (0..=255u8)
        .flat_map(|v| {
            let w = 255 - v;
            [
                [v, 0, 0, 255],
                [0, v, 0, 255],
                [0, 0, v, 255],
                [v, v, v, 255],
                [v, w, v / 2, 255],
                [w / 2, v, w, 255],
            ]
        })
        .flatten()
        .collect();
    let mut out = vec![0; probe.len()];
    transform.transform(&probe, &mut out).is_ok() && out == probe
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Display P3 (D65, the sRGB curve) to sRGB worked by hand: decode, the standard 3 x 3
    /// matrix between the two sets of primaries, clip, encode. Independent of moxcms.
    fn p3_in_srgb(p3: [u8; 3]) -> [u8; 3] {
        const M: [[f64; 3]; 3] = [
            [1.224_940_2, -0.224_940_4, 0.0],
            [-0.042_056_9, 1.042_057_1, 0.0],
            [-0.019_637_6, -0.078_636_1, 1.098_273_5],
        ];
        let linear = p3.map(|v| {
            let c = f64::from(v) / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        });
        M.map(|row| {
            let c = (row[0] * linear[0] + row[1] * linear[1] + row[2] * linear[2]).clamp(0.0, 1.0);
            let encoded = if c <= 0.003_130_8 {
                c * 12.92
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            };
            (encoded * 255.0).round() as u8
        })
    }

    fn display_p3() -> Vec<u8> {
        ColorProfile::new_display_p3().encode().expect("encode")
    }

    #[test]
    fn display_p3_comes_in_as_its_srgb_colour_and_clips_outside() {
        let colours = [
            [200, 100, 50],
            [60, 160, 90],
            [70, 90, 200],
            [128, 128, 128],
            [240, 200, 40],
            [255, 0, 0],
            [0, 255, 0],
        ];
        let mut pixels: Vec<u8> = colours
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 77])
            .collect();
        let profile = display_p3();
        assert_eq!(
            into_srgb(Some(&profile), &mut pixels),
            Interpretation::Converted
        );
        for (got, p3) in pixels.chunks(4).zip(colours) {
            let want = p3_in_srgb(p3);
            for channel in 0..3 {
                assert!(
                    got[channel].abs_diff(want[channel]) <= 1,
                    "{p3:?}: {got:?} against {want:?}"
                );
            }
            assert_eq!(got[3], 77, "alpha stays");
        }
        assert_eq!(p3_in_srgb([200, 100, 50]), [215, 93, 31]);
        assert_eq!(p3_in_srgb([255, 0, 0]), [255, 0, 0], "clipped to sRGB red");
    }

    #[test]
    fn untagged_srgb_and_unusable_profiles_keep_every_byte() {
        let original: Vec<u8> = (0..=255u8).flat_map(|v| [v, 255 - v, v / 3, v]).collect();
        let mut pixels = original.clone();
        assert_eq!(into_srgb(None, &mut pixels), Interpretation::Untagged);
        assert_eq!(
            into_srgb(Some(SRGB_PROFILE), &mut pixels),
            Interpretation::Srgb
        );
        assert_eq!(
            into_srgb(Some(b"not a profile"), &mut pixels),
            Interpretation::Ignored
        );
        let grey = ColorProfile::new_gray_with_gamma(2.2)
            .encode()
            .expect("grey");
        assert_eq!(into_srgb(Some(&grey), &mut pixels), Interpretation::Ignored);
        assert_eq!(pixels, original);
    }

    #[test]
    fn the_export_profile_is_moxcms_srgb() {
        let mut fresh = ColorProfile::new_srgb().encode().expect("encode");
        // Bytes 24 to 35 are the creation time, stamped by each encoding.
        fresh[24..36].copy_from_slice(&SRGB_PROFILE[24..36]);
        assert_eq!(fresh, SRGB_PROFILE);
        let parsed = ColorProfile::new_from_slice(SRGB_PROFILE).expect("parses");
        assert_eq!(parsed.color_space, DataColorSpace::Rgb);
    }
}
