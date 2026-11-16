//! Each kind's description (S109, S122, S123): stored codes read back, every topic described once
//! with a key of its own, what each topic's facts say of it; and every skill's lesson at every
//! level (`lessons.rs`).

mod lessons;

use openpaint_practice::{
    BoxView, Carve, Combination, CurvedRotation, Emphasis, Estimate, Facts, Fill, Guide, Horizon,
    Kind, Lean, Leans, Lightness, LineTier, Lit, LitColour, Mask, MaskShape, Metric, Pen,
    Placement, Purity, Rotation, ScaleTone, Solid, Tendency, Texture, Throw, Tier, Tone, Trick,
};

/// A kind of every variant the stores may hold.
fn every_kind() -> Vec<Kind> {
    let mut kinds: Vec<Kind> = LineTier::ALL.map(Kind::Line).to_vec();
    kinds.extend(Combination::ALL.map(Kind::Combined));
    kinds.extend(BoxView::ALL.map(Kind::Box));
    kinds.extend(Horizon::ALL.map(Kind::FreeBoxes));
    kinds.extend(Guide::ALL.map(Kind::Ellipse));
    kinds.extend(Estimate::ALL.map(Kind::Flat));
    kinds.extend(Trick::ALL.map(Kind::Space));
    kinds.extend(Solid::ALL.map(Kind::Solid));
    kinds.extend(Carve::ALL.map(Kind::Object));
    kinds.extend(Rotation::ALL.map(Kind::Rotation));
    for tier in Tier::ALL {
        let tiered = [Kind::Shape, Kind::Crop, Kind::Division, Kind::WholeCopy];
        kinds.extend(tiered.map(|kind| kind(tier)));
        kinds.push(Kind::ValueMasses(tier));
        kinds.push(Kind::Analysis(tier));
    }
    for tone in ScaleTone::ALL {
        kinds.extend([5, 7, 9].map(|cells| Kind::Scale(tone, cells)));
    }
    kinds.push(Kind::Colour(Tone {
        purity: Purity::Muted,
        lightness: Lightness::Middle,
    }));
    kinds.push(Kind::KeyShift(Tendency {
        lean: Lean::WarmPure,
        hue: 30,
    }));
    kinds.push(Kind::Study);
    kinds.push(Kind::Free);
    kinds.push(Kind::Observation);
    kinds.extend(MaskShape::ALL.map(|shape| Kind::GamutMask(Mask { shape, hue: 60 })));
    kinds.extend(Placement::ALL.map(Kind::Placement));
    kinds.extend(Lit::ALL.map(Kind::Light));
    kinds.extend(Throw::ALL.map(Kind::Shadows));
    kinds.extend(CurvedRotation::ALL.map(Kind::CurvedRotation));
    kinds.extend(LitColour::ALL.map(Kind::LitColour));
    kinds.extend(Texture::ALL.map(Kind::Texture));
    kinds.extend(Emphasis::ALL.map(Kind::Thumbnail));
    kinds.extend([0, 7, 100].map(|n| Kind::ColdWarm(Leans::nth(n))));
    kinds
}

#[test]
fn every_kind_reads_back_from_its_stored_code_and_codes_run_one_to_thirty_two_but_five() {
    let mut codes = Vec::new();
    for kind in every_kind() {
        let (code, variant) = kind.code();
        assert_eq!(Kind::of_code(code, variant), Some(kind), "{kind:?}");
        codes.push(code);
    }
    codes.sort_unstable();
    codes.dedup();
    let expected: Vec<u8> = (1..=32).filter(|code| *code != 5).collect();
    assert_eq!(codes, expected);
    assert_eq!(Kind::Thumbnail(Emphasis::Two).code(), (31, 2));
    let first = Leans::nth(0);
    assert_eq!(Kind::ColdWarm(first).code().0, 32);
    assert_eq!(Kind::of_code(32, 195), None, "13 warm hues by 15 cool");
    assert_eq!(Kind::of_code(5, 0), None, "code 5 is free");
    // The first lesson's free scribble (S122) is kind 20, its one variant 0.
    assert_eq!(Kind::Free.code(), (20, 0));
    // A drawing from life (S151) is kind 23; a gamut mask (S153) 24, its shape times 36 and its
    // hue's step; a placement (S153) 25, light on form 26, cast shadows 27, a curved rotation
    // (S154f) 28, colour under light 29, texture 30 (S154j), a contrast thumbnail 31, a cold-warm
    // copy 32 (S154g); no kind is 33.
    assert_eq!(Kind::Observation.code(), (23, 0));
    let small = Mask {
        shape: MaskShape::Small,
        hue: 60,
    };
    assert_eq!(Kind::GamutMask(small).code(), (24, 2 * 36 + 6));
    assert_eq!(Kind::of_code(24, 108), None);
    assert_eq!(Kind::of_code(33, 0), None);
    assert_eq!(Kind::Analysis(Tier::Hard).code(), (21, 4));
    // Codes pinned for good: points in space, estimates, objects, compositions, crops.
    assert_eq!(Kind::Space(Trick::Mirror).code(), (12, 4));
    assert_eq!(Kind::Flat(Estimate::Angle).code(), (11, 2));
    assert_eq!(Kind::Object(Carve::Round).code(), (14, 3));
    assert_eq!(Kind::Division(Tier::Simple).code(), (15, 2));
    assert_eq!(Kind::WholeCopy(Tier::Medium).code(), (16, 3));
    assert_eq!(Kind::ValueMasses(Tier::Medium).code(), (17, 3));
    assert_eq!(Kind::of_code(18, 3), Some(Kind::Crop(Tier::Medium)));
    assert_eq!((Kind::of_code(18, 0), Kind::of_code(20, 2)), (None, None));
    assert_eq!(
        (Metric::SpaceOff.code(), Metric::MassesMatched.code()),
        (46, 55)
    );
}

/// Every topic's facts, by their keys in order: what every kind's facts are.
fn topics() -> Vec<&'static Facts> {
    let mut topics: Vec<&Facts> = every_kind().into_iter().map(Kind::facts).collect();
    topics.sort_by_key(|facts| facts.key);
    topics.dedup_by_key(|facts| facts.key);
    topics
}

#[test]
fn every_topic_is_described_once_with_a_key_of_its_own() {
    for kind in every_kind() {
        let facts = kind.facts();
        let lowered = |c: char| c.is_ascii_lowercase() || c == '-';
        assert!(facts.key.chars().all(lowered), "{}", facts.key);
        let alike = every_kind()
            .into_iter()
            .filter(|k| k.facts().key == facts.key);
        assert!(alike.into_iter().all(|k| k.facts() == facts));
    }
    // Free boxes looking down and up are topics of their own (S142).
    assert_eq!(topics().len(), 45);
}

#[test]
fn a_topic_s_facts_say_what_it_is() {
    let aligned = ["line", "combined", "shape", "observation"];
    for facts in topics() {
        let key = facts.key;
        assert_eq!(facts.aligned, aligned.contains(&key), "{key}");
        assert_eq!(facts.compared, key != "free", "{key}");
        assert!(facts.seconds >= 30, "{key}: how long one takes");
        let tools = &facts.tools;
        let pens: &[Pen] = match key {
            "division" | "abstract-analysis" => &Pen::DIVIDING,
            "whole-copy" => &Pen::COPYING,
            "value-masses" | "light-halftone" | "contrast-thumbnail" => &Pen::VALUES,
            "light-families" => &Pen::TWO_TONES,
            "light-core" => &Pen::FOUR_TONES,
            "texture" | "texture-sparse" => &Pen::SHADOWS,
            "colour" | "scale" | "combination" | "key-shift" | "study" | "gamut-mask"
            | "cold-warm" => &[],
            "lit-colour" | "lit-colour-bounce" => &[],
            _ => &Pen::DRAWING,
        };
        assert_eq!(tools.pens, pens, "{key}");
        let painted = ["whole-copy", "combination", "key-shift", "gamut-mask"];
        let painted = [painted.as_slice(), &["lit-colour", "lit-colour-bounce"]].concat();
        assert_eq!(tools.paints, painted.contains(&key), "{key}");
        let fill = match key {
            "scale" => Some(Fill::Cell),
            "colour" | "combination" | "key-shift" | "whole-copy" | "value-masses"
            | "gamut-mask" | "contrast-thumbnail" => Some(Fill::Frame),
            "light-families" | "light-halftone" | "light-core" => Some(Fill::Frame),
            "lit-colour" | "lit-colour-bounce" => Some(Fill::Frame),
            _ => None,
        };
        assert_eq!(tools.fill, fill, "{key}");
        let full = key == "study" || key == "cold-warm";
        assert_eq!((tools.full, tools.layered), (full, key == "whole-copy"));
        // Only a cold-warm copy's sheet leaves room beside its frame (S154).
        assert_eq!(facts.beside, key == "cold-warm", "{key}");
    }
    let constructed = topics().into_iter().filter(|facts| facts.constructed);
    // The twenty built from given parts (light on form's three and cast shadows, S153; colour under
    // light's two, S154), the free scribble's one frame (S122), abstract analysis's, its picture
    // under the marks (S112), and a contrast thumbnail's, its shapes printed in it (S154).
    assert_eq!(constructed.count(), 23);
}
