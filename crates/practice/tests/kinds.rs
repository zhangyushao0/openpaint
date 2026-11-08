//! Each kind's description (S109): stored codes read back, every family described once with a key
//! of its own, and what each family's work says of it as the stages had it.

use openpaint_practice::{
    Arrangement, BoxView, Carve, Combination, Estimate, Family, Guide, Horizon, Kind, Lean,
    Lightness, LineTier, Purity, ScaleTone, Solid, Tendency, Tier, Tone, Trick, Work,
};

/// A kind of every variant the stores may hold.
fn every_kind() -> Vec<Kind> {
    let mut kinds: Vec<Kind> = LineTier::ALL.map(Kind::Line).to_vec();
    kinds.extend(Combination::ALL.map(Kind::Combined));
    kinds.extend(Arrangement::ALL.map(Kind::Areas));
    kinds.extend(BoxView::ALL.map(Kind::Box));
    kinds.extend(Horizon::ALL.map(Kind::FreeBoxes));
    kinds.extend(Guide::ALL.map(Kind::Ellipse));
    kinds.extend(Estimate::ALL.map(Kind::Flat));
    kinds.extend(Trick::ALL.map(Kind::Space));
    kinds.extend(Solid::ALL.map(Kind::Solid));
    kinds.extend(Carve::ALL.map(Kind::Object));
    for tier in Tier::ALL {
        let tiered = [Kind::Shape, Kind::Crop, Kind::Division, Kind::WholeCopy];
        kinds.extend(tiered.map(|kind| kind(tier)));
        kinds.push(Kind::ValueMasses(tier));
    }
    for tone in ScaleTone::ALL {
        kinds.extend([5, 9].map(|cells| Kind::Scale(tone, cells)));
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
    kinds
}

#[test]
fn every_kind_reads_back_from_its_stored_code_and_codes_run_one_to_nineteen() {
    let mut codes = Vec::new();
    for kind in every_kind() {
        let (code, variant) = kind.code();
        assert_eq!(Kind::of_code(code, variant), Some(kind), "{kind:?}");
        codes.push(code);
    }
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes, (1..=19).collect::<Vec<u8>>());
    assert_eq!(Kind::of_code(20, 0), None);
}

#[test]
fn every_family_is_described_once_with_a_key_of_its_own() {
    let mut keys = Vec::new();
    for family in Family::ALL {
        let facts = family.facts();
        assert_eq!(facts.family, family);
        assert_eq!(Family::of_key(facts.key), Some(family));
        assert!(
            facts
                .key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-'),
            "{}",
            facts.key
        );
        keys.push(facts.key);
    }
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), Family::ALL.len());
    // The retired generated combinations go by the combinations' family.
    assert_eq!(Kind::Areas(Arrangement::ALL[0]).family(), Family::Areas);
}

#[test]
fn a_family_s_work_says_what_it_is_as_the_stages_had_it() {
    let lines = [Family::Line, Family::Combined, Family::Shape];
    let painted = [
        Family::Areas,
        Family::Scale,
        Family::Study,
        Family::ValueMasses,
        Family::KeyShift,
    ];
    for family in Family::ALL {
        let work = family.facts().work;
        assert_eq!(lines.contains(&family), work == Work::Lines, "{family:?}");
        assert_eq!(painted.contains(&family), family.painted(), "{family:?}");
        assert_eq!(
            family.drawn(),
            !family.painted() && family != Family::Colour,
            "{family:?}"
        );
        let divided = matches!(family, Family::Division | Family::WholeCopy);
        assert_eq!(family.divided(), divided, "{family:?}");
        assert_eq!(
            family.geometric(),
            family.drawn() && !lines.contains(&family) && !divided,
            "{family:?}"
        );
        let composed = divided || family == Family::ValueMasses;
        assert_eq!(family.composed(), composed, "{family:?}");
    }
    let constructed: Vec<Family> = (Family::ALL.into_iter())
        .filter(|f| f.constructed())
        .collect();
    assert_eq!(constructed.len(), 12, "{constructed:?}");
}
