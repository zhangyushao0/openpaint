//! What a sheet printed, as bytes (S120a): every kind's givens and painted picture round-trip
//! exactly for many seeds (five where its generator paints its own picture, S154), alone and as a
//! sheet's blob. Prints each kind's encoded size and a 36-exercise sheet's of the largest kind
//! laid (`--nocapture`).

use std::collections::BTreeMap;
use std::sync::Arc;

use openpaint_practice::{
    Exercise, Given, Givens, ItemRef, Kind, Masses, Pack, PackItem, PictureKind, SheetPicture,
    Tier, ValueMasses,
};
use openpaint_practice_store::{
    decode_givens, decode_marks, decode_painted, encode_givens, encode_marks, encode_painted,
};

/// The landscapes in the test pack; the exercises printed of each kind's variant: many, and fewer
/// where its generator paints its own picture (a lit form, a texture's strip: a second or more of
/// rendering each).
const SEEDS: u64 = 20;
const SLOTS: u16 = 20;
const PAINTED_SLOTS: u16 = 5;
const PACK: &str = "test-compositions";

/// Every kind a record may hold: each kind code's every stored variant.
fn every_kind() -> Vec<Kind> {
    (1..=u8::MAX)
        .flat_map(|code| (0..=u8::MAX).filter_map(move |variant| Kind::of_code(code, variant)))
        .collect()
}

/// A pack of `SEEDS` landscapes with their value masses, as a composition's pack holds them.
fn compositions() -> Pack {
    let items = (0..SEEDS)
        .map(|seed| {
            let (size, pixels) = Masses::generate(seed, 0.75).picture(320);
            PackItem {
                id: format!("land-{seed}"),
                kind: PictureKind::Composition,
                tier: Tier::Medium,
                size,
                title: format!("Landscape {seed}"),
                credit: None,
                outline: None,
                masses: Some(Arc::new(ValueMasses::of(size, &pixels))),
                areas: None,
            }
        })
        .collect();
    Pack {
        id: PACK.to_owned(),
        title: "Test compositions".to_owned(),
        items,
    }
}

/// What exercise `seed` of `kind` prints, as `Given::printed` takes it, its picture one of
/// `packs`' where its kind copies one: generated once, again only with the picture it copies; and
/// whether its generator painted its picture itself.
fn printed(kind: Kind, seed: u64, packs: &[Pack]) -> (Given, bool) {
    let mut exercise = Exercise::generated(kind, seed);
    let mut original = exercise.original();
    if original.picture().is_some() {
        exercise.picture = Some(ItemRef {
            pack: PACK.to_owned(),
            item: format!("land-{}", seed % SEEDS),
        });
        original = exercise.original();
    }
    let picture = (exercise.picture.as_ref()).and_then(|item| SheetPicture::of(item, packs));
    let given = Given {
        marks: original.printed(picture.as_ref()),
        painted: original.painted(picture.as_ref()),
    };
    let own = given.painted.is_some() && exercise.picture.is_none();
    (given, own)
}

/// `given` round-tripped alone and in a sheet's blob; its bytes in the blob (without the
/// sheet's format and count).
fn round_trip(kind: Kind, slot: u16, given: &Given) -> usize {
    assert_eq!(
        decode_marks(&encode_marks(&given.marks)).as_ref(),
        Ok(&given.marks)
    );
    if let Some(picture) = &given.painted {
        assert_eq!(
            decode_painted(&encode_painted(picture)).as_ref(),
            Ok(picture)
        );
    }
    let sheet = Givens {
        slots: vec![(slot, given.clone())],
    };
    let bytes = encode_givens(&sheet);
    assert_eq!(decode_givens(&bytes), Ok(sheet), "{kind:?}");
    bytes.len() - 5
}

#[test]
fn every_kind_s_givens_and_painted_picture_round_trip_exactly() {
    let packs = [compositions()];
    let kinds = every_kind();
    assert!(kinds.len() > 40, "{}", kinds.len());
    let mut painted = 0;
    let mut sizes = BTreeMap::<u8, (String, Vec<usize>)>::new();
    let mut by_kind: Vec<(Kind, usize)> = Vec::new();
    for kind in kinds {
        let seed = |slot: u16| 0x5EED_0120_0000 + u64::from(slot);
        let (first, own) = printed(kind, seed(0), &packs);
        let slots = if own { PAINTED_SLOTS } else { SLOTS };
        let rest = (1..slots).map(|slot| printed(kind, seed(slot), &packs).0);
        let mut total = 0;
        for (slot, given) in (0..slots).zip(std::iter::once(first).chain(rest)) {
            painted += usize::from(given.painted.is_some());
            let bytes = round_trip(kind, slot, &given);
            total += bytes;
            let name = format!("{kind:?}");
            let name = name.split('(').next().unwrap_or_default().to_owned();
            let row = sizes.entry(kind.code().0).or_insert((name, Vec::new()));
            row.1.push(bytes);
        }
        by_kind.push((kind, total / usize::from(slots)));
    }
    assert!(painted >= 60, "objects, value masses, combinations painted");
    print_sizes(&sizes);
    let (largest, _) = (by_kind.iter())
        .max_by_key(|(_, mean)| *mean)
        .expect("kinds");
    sheet_of(*largest, &packs);
}

/// Each kind's bytes of one exercise, mean and largest.
fn print_sizes(sizes: &BTreeMap<u8, (String, Vec<usize>)>) {
    println!("| kind (code) | exercises | mean bytes | largest bytes |");
    println!("| --- | --- | --- | --- |");
    for (code, (name, bytes)) in sizes {
        let mean = bytes.iter().sum::<usize>() / bytes.len();
        let largest = bytes.iter().max().copied().unwrap_or(0);
        println!("| {name} ({code}) | {} | {mean} | {largest} |", bytes.len());
    }
}

/// A sheet of 36 exercises of `kind`, round-tripped and its bytes printed.
fn sheet_of(kind: Kind, packs: &[Pack]) {
    let sheet = Givens {
        slots: (0..36_u16)
            .map(|slot| (slot, printed(kind, u64::from(slot), packs).0))
            .collect(),
    };
    let bytes = encode_givens(&sheet);
    assert_eq!(decode_givens(&bytes).as_ref(), Ok(&sheet));
    println!("a sheet of 36 {kind:?}: {} bytes", bytes.len());
    let last = &sheet.slots[35].1;
    assert_eq!(sheet.of(35), Some(last));
    assert_eq!(sheet.of(36), None);
}
