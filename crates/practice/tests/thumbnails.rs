//! Contrast thumbnails (S154): their rings printed on a composition's shapes where each level
//! asks, the rules read from a thumbnail's own pixels each firing alone on a copy made wrong on
//! purpose, the veil clear where the copy is strongest, the eye judging once the frame is covered.

use std::sync::Arc;

use openpaint_practice::{
    Asked, CardItem, Choices, Compare, Compared, Comparison, Copied, DrawnCopy, Emphasis, Exercise,
    Figure, Finding, ItemRef, Kind, Level, LinePen, MOVED_BY, Masses, MassesShown, PictureFit,
    Point, RING, Reads, Rgb8, Ring, SheetPicture, Steps, ThumbnailCompared, ThumbnailRead,
    ThumbnailReading, VEILED, Value, ValueMasses, Verdict, distance, rings,
};

/// The frame read: 400 by 300 pixels, its extent in frame units.
const FRAME: [u32; 2] = [400, 300];
const EXTENT: Point = [1.0, 0.75];
/// The first ring and the second, on the cells' grid (a cell is 0.005 frame units).
const RED: Point = [0.3, 0.35];
const GREEN: Point = [0.75, 0.4];
/// A place outside both rings.
const ELSEWHERE: Point = [0.75, 0.15];

fn ring(centre: Point, second: bool) -> Ring {
    Ring {
        centre,
        radius: RING,
        second,
    }
}

/// What a motif paints at a frame point: dark in its middle on a light disc (two values), or a
/// light disc alone (one value), round `centre`; none elsewhere.
#[derive(Clone, Copy)]
enum Motif {
    DarkOnLight(Point),
    Light(Point),
}

impl Motif {
    fn at(self, p: Point) -> Option<Value> {
        match self {
            Self::DarkOnLight(centre) if distance(p, centre) <= 0.035 => Some(Value::Dark),
            Self::DarkOnLight(centre) | Self::Light(centre) if distance(p, centre) <= 0.07 => {
                Some(Value::Light)
            }
            _ => None,
        }
    }
}

/// A thumbnail filled with the middle grey and `motifs` painted over it, as its frame's pixels.
fn painted(motifs: &[Motif]) -> Vec<u8> {
    let unit = f64::from(FRAME[0]);
    (0..FRAME[0] * FRAME[1])
        .flat_map(|at| {
            let p = [at % FRAME[0], at / FRAME[0]].map(|v| (f64::from(v) + 0.5) / unit);
            let value = motifs.iter().find_map(|motif| motif.at(p));
            let [r, g, b] = value.unwrap_or(Value::Middle).grey();
            [r, g, b, 255]
        })
        .collect()
}

/// A thumbnail made wrong on purpose: what it is, its motifs, its rings and what the rules find.
type Case<'a> = (&'a str, Vec<Motif>, &'a [Ring], Vec<Finding>);

fn read(motifs: &[Motif], rings: &[Ring]) -> ThumbnailRead {
    ThumbnailRead::of(&Steps::of(FRAME, &painted(motifs)), rings, EXTENT)
}

#[test]
fn each_rule_fires_alone_on_a_thumbnail_made_wrong_on_purpose() {
    use Motif::{DarkOnLight, Light};
    let one = [ring(RED, false)];
    let two = [ring(RED, false), ring(GREEN, true)];
    let cases: [Case<'_>; 6] = [
        ("the ring strongest", vec![DarkOnLight(RED)], &one, vec![]),
        (
            "two values outside, one in the ring",
            vec![Light(RED), DarkOnLight(ELSEWHERE)],
            &one,
            vec![Finding::Outside],
        ),
        (
            "as strong outside as in the ring",
            vec![DarkOnLight(RED), DarkOnLight(ELSEWHERE)],
            &one,
            vec![Finding::Rival],
        ),
        (
            "the second clearly weaker, the rest weaker still",
            vec![DarkOnLight(RED), Light(GREEN)],
            &two,
            vec![],
        ),
        (
            "the second as strong as the first",
            vec![DarkOnLight(RED), DarkOnLight(GREEN)],
            &two,
            vec![Finding::SecondStrong],
        ),
        (
            "a place outside as strong as the second",
            vec![DarkOnLight(RED), Light(GREEN), Light(ELSEWHERE)],
            &two,
            vec![Finding::SecondWeak],
        ),
    ];
    for (what, motifs, rings, findings) in cases {
        let read = read(&motifs, rings);
        assert_eq!(read.findings, findings, "{what}");
        assert!(read.bare < 0.01, "{what}: filled");
    }
    // What is a place is ringed there: the strongest step outside, at the motif.
    let rival = read(&[DarkOnLight(RED), DarkOnLight(ELSEWHERE)], &one);
    assert!(rival.places.len() == 1 && distance(rival.places[0], ELSEWHERE) < 0.1);
}

#[test]
fn the_veil_is_clear_where_the_thumbnail_is_strongest_and_the_eye_judges_it_once_covered() {
    let reading = ThumbnailReading {
        rings: vec![ring(RED, false)],
        extent: EXTENT,
    };
    let unread = Reads::unread(&reading);
    assert_eq!(unread.verdict(), Verdict::Incomplete, "not read yet");
    let pixels = painted(&[Motif::DarkOnLight(RED)]);
    let read = Reads::read(&reading, FRAME, &pixels, [255; 3], None);
    assert!(read.metrics.is_empty(), "no number kept");
    let [(size, veil)] = &read.pictures[..] else {
        panic!("one picture: the veil");
    };
    let alpha = |[x, y]: Point| {
        let cell = |v: f64, side: u32| ((v * f64::from(size[0])) as usize).min(side as usize - 1);
        veil[4 * (cell(y, size[1]) * size[0] as usize + cell(x, size[0])) + 3]
    };
    let edge = [RED[0] + 0.035, RED[1]];
    assert!(
        alpha(edge) < 40,
        "clear at the dark's edge: {}",
        alpha(edge)
    );
    let flat = alpha([0.9, 0.65]);
    assert_eq!(flat, (VEILED * 255.0).round() as u8, "veiled where flat");
    assert_eq!(read.comparison.verdict(), Verdict::Eye);
    let compared = read
        .comparison
        .downcast::<ThumbnailCompared>()
        .expect("read");
    let seen = seen();
    let marks = compared.marks(&seen);
    assert!(
        matches!(marks[0].drawn, Figure::Picture { .. }),
        "the veil first"
    );
    assert!(compared.card(&seen).is_empty(), "nothing found");
    // Left bare: no verdict honest, the card says so.
    let bare = [255_u8; 4].repeat((FRAME[0] * FRAME[1]) as usize);
    let left = Reads::read(&reading, FRAME, &bare, [255; 3], None);
    assert_eq!(left.comparison.verdict(), Verdict::Incomplete);
    let said = left.comparison.card(&seen);
    assert!(matches!(&said[..], [CardItem::Says(say)] if say.key == "thumbnail-bare"));
}

fn seen() -> openpaint_practice::Seen<'static> {
    openpaint_practice::Seen {
        construction: false,
        shown: 0.5,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    }
}

/// A landscape's value masses, 4 by 3.
fn landscape() -> ValueMasses {
    let (size, pixels) = Masses::generate(0x5154, 0.75).picture(400);
    ValueMasses::of(size, &pixels)
}

#[test]
fn the_rings_lie_on_the_shapes_where_each_level_asks_the_same_every_time() {
    let masses = landscape();
    let [own] = rings(&masses, EXTENT, Emphasis::Own, 3)[..] else {
        panic!("one ring");
    };
    let steps = Steps::of(masses.size, &masses.picture());
    let (most, _) = steps.strongest(|_| true).expect("steps");
    let at = |p: Point| steps.strongest(|q| distance([q[0], q[1] * 0.75], p) < 0.01);
    let here = at(own.centre).map_or(0.0, |(step, _)| step);
    assert!(
        here > most - 0.05,
        "the composition's own focus: {here} of {most}"
    );
    for seed in 0..12 {
        let [moved] = rings(&masses, EXTENT, Emphasis::Moved, seed)[..] else {
            panic!("one ring");
        };
        assert!(
            distance(moved.centre, own.centre) >= MOVED_BY - 1e-9,
            "{seed}"
        );
        let two = rings(&masses, EXTENT, Emphasis::Two, seed);
        assert_eq!(
            two.iter().map(|ring| ring.second).collect::<Vec<_>>(),
            [false, true]
        );
        assert!(distance(two[0].centre, two[1].centre) >= MOVED_BY - 1e-9);
        for ring in std::iter::once(moved).chain(two) {
            let [x, y] = ring.centre;
            let inside = (RING..=1.0 - RING).contains(&x) && (RING..=0.75 - RING).contains(&y);
            assert!(inside, "{ring:?}");
            let on = at(ring.centre).map_or(0.0, |(step, _)| step);
            assert!(on >= 0.1, "on an edge between masses: {on}");
        }
        assert_eq!(
            rings(&masses, EXTENT, Emphasis::Two, seed),
            rings(&masses, EXTENT, Emphasis::Two, seed)
        );
    }
}

/// A copy with paint in its frame.
struct Held;

impl Copied for Held {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        Vec::new()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy::default()
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        Vec::new()
    }
}

/// The landscape `masses` as the sheet holds a composition of `item`, 800 by 600.
fn sheet_picture(masses: &Arc<ValueMasses>, item: ItemRef) -> SheetPicture {
    let fit = PictureFit::of([800, 600]);
    SheetPicture {
        item,
        size: [800, 600],
        fit,
        outline: None,
        credit: None,
        masses: Some(MassesShown {
            edges: (masses.edges().into_iter())
                .map(|edge| {
                    edge.iter()
                        .map(|[u, v]| [u * fit.size[0], v * fit.size[1]])
                        .collect()
                })
                .collect(),
            answer: Arc::clone(masses),
        }),
        areas: None,
    }
}

#[test]
fn it_prints_its_shapes_faint_and_its_rings_and_is_read_against_them() {
    let masses = Arc::new(landscape());
    let item = ItemRef {
        pack: "test-compositions".to_owned(),
        item: "harbour".to_owned(),
    };
    let picture = sheet_picture(&masses, item.clone());
    let exercise = Exercise {
        picture: Some(item),
        ..Exercise::generated(Kind::Thumbnail(Emphasis::Two), 11)
    };
    let original = exercise.original();
    assert!(original.givens().is_empty(), "nothing without its picture");
    let printed = original.printed(Some(&picture));
    let pens: Vec<(LinePen, bool)> = (printed.iter())
        .filter_map(|mark| match &mark.drawn {
            Figure::Line { pen, dashed, .. } => Some((*pen, *dashed)),
            _ => None,
        })
        .collect();
    let edges = picture.masses.as_ref().map_or(0, |m| m.edges.len());
    assert_eq!(pens.len(), edges + 2);
    assert!(
        pens[..edges]
            .iter()
            .all(|pen| *pen == (LinePen::Faint, false))
    );
    assert_eq!(
        pens[edges..],
        [
            (LinePen::Level(Level::Main), true),
            (LinePen::Level(Level::Second), true)
        ]
    );
    let asked = Asked {
        exercise: &exercise,
        original: &*original,
        picture: Some(&picture),
        copy: &Held,
        choices: Choices::default(),
    };
    let Some(Compare::Read(reading)) = original.compare(&asked) else {
        panic!("read from its pixels");
    };
    let reading = reading
        .downcast::<ThumbnailReading>()
        .expect("a thumbnail's");
    let fitted = rings(&masses, picture.fit.size, Emphasis::Two, 11);
    assert_eq!(reading.rings, fitted);
    let _: Comparison = Reads::unread(reading);
}
