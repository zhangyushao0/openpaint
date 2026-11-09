//! A sheet's givens as bytes (S120a), variant by variant: every figure, frame, clip, pen, source
//! and place round-trips to the bit, and bytes cut short, longer or holding a code this build does
//! not know are refused, never guessed.

use openpaint_app::PortError;
use openpaint_practice::{
    Clip, Figure, In, Level, LinePen, Painted, Place, Point, SheetMark, Source, Value,
};
use openpaint_practice_store::{
    GIVENS_FORMAT, Given, Givens, decode_givens, decode_marks, decode_painted, encode_givens,
    encode_marks, encode_painted,
};

/// Points at a float's edges: -0.0, the largest, a subnormal.
const A: Point = [0.1, -0.0];
const B: Point = [f64::MAX, f64::MIN_POSITIVE / 2.0];

const CORRUPT: Option<PortError> = Some(PortError::Corrupt {
    what: "practice givens",
});

/// A line in every pen, dashed and whole in turn, through an infinite point.
fn pen_lines() -> Vec<Figure> {
    let pens = [
        LinePen::Ink,
        LinePen::Printed,
        LinePen::PrintedHalf,
        LinePen::Answer,
        LinePen::Laid,
        LinePen::Off,
        LinePen::Faint,
        LinePen::Pencil,
    ];
    let pens = (pens.into_iter())
        .chain(Level::ALL.map(LinePen::Level))
        .chain(Value::ALL.map(LinePen::Hatch));
    (pens.enumerate())
        .map(|(n, pen)| Figure::Line {
            points: vec![A, B, [0.5, f64::INFINITY]],
            pen,
            dashed: n % 2 == 0,
        })
        .collect()
}

/// Every other figure, and a picture from every source, in either place.
fn figures() -> Vec<Figure> {
    let mut figures = vec![
        Figure::Ring(A),
        Figure::Found(B),
        Figure::Dot { at: A, small: true },
        Figure::Given {
            at: B,
            pen: LinePen::Printed,
        },
        Figure::Counted {
            at: A,
            radius: 1e-300,
            pen: LinePen::Hatch(Value::Middle),
        },
        Figure::Narrow {
            at: A,
            legs: [B, A],
        },
        Figure::Shade(vec![A, B, A]),
        Figure::Shade(Vec::new()),
        Figure::Horizon(f64::NEG_INFINITY),
        Figure::Swatch([1, 2, 3]),
        Figure::Fill {
            corners: vec![B, A],
            colour: [255, 0, 128],
        },
        Figure::Patch([0, 0, 0]),
        Figure::Cell {
            cell: usize::MAX,
            rect: [A, B],
        },
        Figure::Rule([B, A]),
        Figure::Uneven([A, B]),
        Figure::Haloed {
            lines: vec![vec![A, B], Vec::new(), vec![B]],
            red: true,
        },
        Figure::Pictured,
        Figure::Tendency([200, 100, 50]),
    ];
    let sources = [
        Source::Pack,
        Source::Painted,
        Source::PaintedOrPack,
        Source::GreyOriginal,
        Source::GreyCopy,
    ];
    let shown = [0.0, 1.0, 0.35, f32::MIN_POSITIVE, -0.0];
    for (n, (source, shown)) in sources.into_iter().zip(shown).enumerate() {
        let place = if n % 2 == 0 {
            Place::Frame
        } else {
            Place::Corners([A, B, [0.5, 0.25], [-1.0, 3.0]])
        };
        figures.push(Figure::Picture {
            source,
            place,
            shown,
        });
    }
    figures
}

/// A mark of every figure, frame, clip, pen, source and place.
fn every_variant() -> Vec<SheetMark> {
    let clips = [
        Clip::Free,
        Clip::Laid,
        Clip::Sheet,
        Clip::KeptOff,
        Clip::Inside,
    ];
    (pen_lines().into_iter().chain(figures()).enumerate())
        .map(|(n, drawn)| SheetMark {
            on: [In::Original, In::Copy][n % 2],
            drawn,
            clip: clips[n % clips.len()],
        })
        .collect()
}

/// A small picture of three runs, the first across its rows, the second one pixel.
fn picture() -> Painted {
    let mut pixels: Vec<u8> = [[255, 255, 255, 0]; 7].concat();
    pixels.extend([9, 8, 7, 255]);
    pixels.extend([[30, 40, 50, 128]; 4].concat());
    ([4, 3], pixels)
}

fn rich() -> Givens {
    let marked = Given {
        marks: every_variant(),
        painted: Some(picture()),
    };
    let empty = Given {
        marks: Vec::new(),
        painted: Some(([0, 0], Vec::new())),
    };
    Givens {
        slots: vec![(3, marked), (0, Given::default()), (u16::MAX, empty)],
    }
}

#[test]
fn every_variant_round_trips_bit_for_bit() {
    let marks = every_variant();
    let bytes = encode_marks(&marks);
    let back = decode_marks(&bytes).expect("decoded");
    assert_eq!(back, marks);
    assert_eq!(
        encode_marks(&back),
        bytes,
        "-0.0 and subnormals kept to the bit"
    );
    assert_eq!(decode_painted(&encode_painted(&picture())), Ok(picture()));
    let givens = rich();
    let blob = encode_givens(&givens);
    assert_eq!(blob[0], GIVENS_FORMAT);
    assert_eq!(decode_givens(&blob).as_ref(), Ok(&givens));
    let painted = givens.of(u16::MAX).and_then(|given| given.painted.clone());
    assert_eq!(painted, Some(([0, 0], Vec::new())));
    // NaN is no value a mark compares equal to, but its bits come back as written.
    let nan = f64::from_bits(0x7FF8_0000_DEAD_BEEF);
    let horizon = vec![SheetMark::free(In::Copy, Figure::Horizon(nan))];
    let back = decode_marks(&encode_marks(&horizon)).expect("decoded");
    let Figure::Horizon(height) = back[0].drawn else {
        panic!("a horizon");
    };
    assert_eq!(height.to_bits(), nan.to_bits());
}

#[test]
fn bytes_cut_at_any_length_or_longer_are_refused() {
    let blob = encode_givens(&rich());
    for end in 0..blob.len() {
        assert!(decode_givens(&blob[..end]).is_err(), "givens cut at {end}");
    }
    let marks = encode_marks(&every_variant());
    for end in 0..marks.len() {
        assert!(decode_marks(&marks[..end]).is_err(), "marks cut at {end}");
    }
    let painted = encode_painted(&picture());
    for end in 0..painted.len() {
        assert!(
            decode_painted(&painted[..end]).is_err(),
            "picture cut at {end}"
        );
    }
    let longer = |mut bytes: Vec<u8>| {
        bytes.push(0);
        bytes
    };
    assert_eq!(
        decode_givens(&longer(blob)).err(),
        CORRUPT,
        "a byte past the end"
    );
    assert_eq!(decode_marks(&longer(marks)).err(), CORRUPT);
    assert_eq!(decode_painted(&longer(painted)).err(), CORRUPT);
}

/// `bytes` with byte `at` set to `value`.
fn patched(bytes: &[u8], at: usize, value: u8) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    bytes[at] = value;
    bytes
}

/// `drawn` in the copy as one mark's bytes: its count (4 bytes), frame, clip, figure's code
/// (byte 6), then its fields from byte 7.
fn one(drawn: Figure) -> Vec<u8> {
    encode_marks(&[SheetMark::free(In::Copy, drawn)])
}

#[test]
fn a_mark_s_unknown_code_is_refused_never_guessed() {
    let pictured = one(Figure::Pictured);
    assert_eq!(decode_marks(&pictured).map(|m| m.len()), Ok(1));
    let refused =
        |bytes: &[u8], at: usize, value: u8| decode_marks(&patched(bytes, at, value)).err();
    assert_eq!(refused(&pictured, 4, 2), CORRUPT, "a frame");
    assert_eq!(refused(&pictured, 5, 5), CORRUPT, "a clip");
    assert_eq!(refused(&pictured, 6, 19), CORRUPT, "a figure");
    assert_eq!(refused(&pictured, 6, u8::MAX), CORRUPT, "a figure");
    let given = one(Figure::Given {
        at: [0.5; 2],
        pen: LinePen::Ink,
    });
    assert_eq!(refused(&given, 7 + 16, 14), CORRUPT, "a pen");
    let laid = one(Figure::Picture {
        source: Source::Pack,
        place: Place::Frame,
        shown: 1.0,
    });
    assert_eq!(refused(&laid, 7, 5), CORRUPT, "a source");
    assert_eq!(refused(&laid, 8, 2), CORRUPT, "a place");
    let dot = one(Figure::Dot {
        at: [0.0; 2],
        small: false,
    });
    assert_eq!(refused(&dot, 7 + 16, 2), CORRUPT, "a flag");
}

#[test]
fn a_sheet_s_or_a_picture_s_unknown_code_is_refused_never_guessed() {
    // A sheet: its format, the slot count (4 bytes), a slot's number (2), marks and picture flag.
    let sheet = encode_givens(&Givens {
        slots: vec![(1, Given::default())],
    });
    assert_eq!(sheet.len(), 1 + 4 + 2 + 4 + 1);
    let flag = decode_givens(&patched(&sheet, 11, 2));
    assert_eq!(flag.err(), CORRUPT, "a picture's flag");
    let other = decode_givens(&patched(&sheet, 0, GIVENS_FORMAT + 1));
    let found = Some(u32::from(GIVENS_FORMAT + 1));
    assert_eq!(other, Err(PortError::OtherFormat { found }));

    // A picture: its size (8 bytes), then runs of a varint length and four bytes.
    let small = encode_painted(&picture());
    assert_eq!(small.len(), 8 + 3 * 5, "three runs");
    let refused = |at: usize, value: u8| decode_painted(&patched(&small, at, value)).err();
    assert_eq!(refused(8, 0), CORRUPT, "a run of nothing");
    assert_eq!(refused(8, 13), CORRUPT, "a run past the end");
    assert_eq!(refused(1, 0x10), CORRUPT, "a side past a pack's");
}
