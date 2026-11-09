//! Sheet marks as bytes (S120a), lossless: a mark is its frame (u8, `In`), its clip (u8, `Clip`)
//! and its figure, a code (u8) and its fields in order. A point is its x and y (f64 each, their
//! 64 bits), a list of points a count (u32) and the points, a colour three u8 (sRGB), a strength
//! (`Picture::shown`) its 32 bits, a cell's index a u64. A pen is one u8 (a level's and a hatch's
//! value among them). Every code is stable: a variant is appended, never renumbered; a code this
//! build does not know is corrupt, never a guess.

use openpaint_app::PortError;
use openpaint_practice::{
    Clip, Figure, In, Level, LinePen, Place, Point, Rgb8, SheetMark, Source, Value,
};
use openpaint_store::{Reader, Writer};

/// The fewest bytes of a mark (its frame, clip and figure's code), of a point and of a line.
const MARK_BYTES: usize = 3;
const POINT_BYTES: usize = 16;
const LINE_BYTES: usize = 4;

/// The frames, clips, pens and sources by their codes.
const INS: [In; 2] = [In::Original, In::Copy];
const CLIPS: [Clip; 5] = [
    Clip::Free,
    Clip::Laid,
    Clip::Sheet,
    Clip::KeptOff,
    Clip::Inside,
];
const PENS: [LinePen; 14] = [
    LinePen::Ink,
    LinePen::Printed,
    LinePen::PrintedHalf,
    LinePen::Answer,
    LinePen::Laid,
    LinePen::Off,
    LinePen::Faint,
    LinePen::Pencil,
    LinePen::Level(Level::Main),
    LinePen::Level(Level::Second),
    LinePen::Level(Level::Third),
    LinePen::Hatch(Value::Dark),
    LinePen::Hatch(Value::Middle),
    LinePen::Hatch(Value::Light),
];
const SOURCES: [Source; 5] = [
    Source::Pack,
    Source::Painted,
    Source::PaintedOrPack,
    Source::GreyOriginal,
    Source::GreyCopy,
];

/// The figures' codes: the drawn ones below `COLOURED`, the coloured and laid ones from it.
const LINE: u8 = 0;
const RING: u8 = 1;
const FOUND: u8 = 2;
const DOT: u8 = 3;
const GIVEN: u8 = 4;
const COUNTED: u8 = 5;
const NARROW: u8 = 6;
const SHADE: u8 = 7;
const HORIZON: u8 = 8;
const RULE: u8 = 9;
const UNEVEN: u8 = 10;
const HALOED: u8 = 11;
const COLOURED: u8 = 12;
const SWATCH: u8 = 12;
const FILL: u8 = 13;
const PATCH: u8 = 14;
const CELL: u8 = 15;
const PICTURE: u8 = 16;
const PICTURED: u8 = 17;
const TENDENCY: u8 = 18;

pub(crate) fn write_marks(writer: &mut Writer, marks: &[SheetMark]) {
    writer.len(marks.len());
    for mark in marks {
        writer.u8(match mark.on {
            In::Original => 0,
            In::Copy => 1,
        });
        writer.u8(match mark.clip {
            Clip::Free => 0,
            Clip::Laid => 1,
            Clip::Sheet => 2,
            Clip::KeptOff => 3,
            Clip::Inside => 4,
        });
        write_figure(writer, &mark.drawn);
    }
}

pub(crate) fn read_marks(reader: &mut Reader<'_>) -> Result<Vec<SheetMark>, PortError> {
    let count = reader.count(MARK_BYTES)?;
    let mut marks = Vec::with_capacity(count);
    for _ in 0..count {
        let on = coded(reader, &INS)?;
        let clip = coded(reader, &CLIPS)?;
        let drawn = read_figure(reader)?;
        marks.push(SheetMark { on, drawn, clip });
    }
    Ok(marks)
}

/// `figure`'s code and then its fields.
fn write_figure(writer: &mut Writer, figure: &Figure) {
    writer.u8(figure_code(figure));
    match figure {
        Figure::Line {
            points,
            pen,
            dashed,
        } => {
            write_points(writer, points);
            write_pen(writer, *pen);
            writer.bool(*dashed);
        }
        Figure::Ring(at) | Figure::Found(at) => write_point(writer, *at),
        Figure::Dot { at, small } => {
            write_point(writer, *at);
            writer.bool(*small);
        }
        Figure::Given { at, pen } => {
            write_point(writer, *at);
            write_pen(writer, *pen);
        }
        Figure::Counted { at, radius, pen } => {
            write_point(writer, *at);
            writer.f64(*radius);
            write_pen(writer, *pen);
        }
        Figure::Narrow { at, legs } => {
            write_point(writer, *at);
            legs.iter().for_each(|leg| write_point(writer, *leg));
        }
        Figure::Shade(points) => write_points(writer, points),
        Figure::Horizon(height) => writer.f64(*height),
        Figure::Rule(ends) | Figure::Uneven(ends) => {
            ends.iter().for_each(|end| write_point(writer, *end));
        }
        Figure::Haloed { lines, red } => {
            writer.len(lines.len());
            lines.iter().for_each(|line| write_points(writer, line));
            writer.bool(*red);
        }
        Figure::Swatch(colour) | Figure::Patch(colour) | Figure::Tendency(colour) => {
            writer.bytes(colour);
        }
        Figure::Fill { corners, colour } => {
            write_points(writer, corners);
            writer.bytes(colour);
        }
        Figure::Cell { cell, rect } => {
            writer.u64(*cell as u64);
            rect.iter().for_each(|corner| write_point(writer, *corner));
        }
        Figure::Picture {
            source,
            place,
            shown,
        } => write_picture(writer, (*source, place, *shown)),
        Figure::Pictured => {}
    }
}

const fn figure_code(figure: &Figure) -> u8 {
    match figure {
        Figure::Line { .. } => LINE,
        Figure::Ring(_) => RING,
        Figure::Found(_) => FOUND,
        Figure::Dot { .. } => DOT,
        Figure::Given { .. } => GIVEN,
        Figure::Counted { .. } => COUNTED,
        Figure::Narrow { .. } => NARROW,
        Figure::Shade(_) => SHADE,
        Figure::Horizon(_) => HORIZON,
        Figure::Rule(_) => RULE,
        Figure::Uneven(_) => UNEVEN,
        Figure::Haloed { .. } => HALOED,
        Figure::Swatch(_) => SWATCH,
        Figure::Fill { .. } => FILL,
        Figure::Patch(_) => PATCH,
        Figure::Cell { .. } => CELL,
        Figure::Picture { .. } => PICTURE,
        Figure::Pictured => PICTURED,
        Figure::Tendency(_) => TENDENCY,
    }
}

fn read_figure(reader: &mut Reader<'_>) -> Result<Figure, PortError> {
    let code = reader.u8()?;
    if code < COLOURED {
        read_drawn(code, reader)
    } else {
        read_coloured(code, reader)
    }
}

/// A figure drawn in a pen or the paper's red, its code `code`.
fn read_drawn(code: u8, reader: &mut Reader<'_>) -> Result<Figure, PortError> {
    Ok(match code {
        LINE => {
            let (points, pen, dashed) = (read_points(reader)?, read_pen(reader)?, reader.bool()?);
            Figure::Line {
                points,
                pen,
                dashed,
            }
        }
        RING => Figure::Ring(read_point(reader)?),
        FOUND => Figure::Found(read_point(reader)?),
        DOT => {
            let (at, small) = (read_point(reader)?, reader.bool()?);
            Figure::Dot { at, small }
        }
        GIVEN => {
            let (at, pen) = (read_point(reader)?, read_pen(reader)?);
            Figure::Given { at, pen }
        }
        COUNTED => {
            let (at, radius, pen) = (read_point(reader)?, reader.f64()?, read_pen(reader)?);
            Figure::Counted { at, radius, pen }
        }
        NARROW => {
            let at = read_point(reader)?;
            let legs = [read_point(reader)?, read_point(reader)?];
            Figure::Narrow { at, legs }
        }
        SHADE => Figure::Shade(read_points(reader)?),
        HORIZON => Figure::Horizon(reader.f64()?),
        RULE => Figure::Rule([read_point(reader)?, read_point(reader)?]),
        UNEVEN => Figure::Uneven([read_point(reader)?, read_point(reader)?]),
        HALOED => {
            let count = reader.count(LINE_BYTES)?;
            let lines = (0..count).map(|_| read_points(reader));
            let lines = lines.collect::<Result<Vec<_>, _>>()?;
            Figure::Haloed {
                lines,
                red: reader.bool()?,
            }
        }
        _ => return Err(reader.corrupt()),
    })
}

/// A figure of a colour or a picture, its code `code`.
fn read_coloured(code: u8, reader: &mut Reader<'_>) -> Result<Figure, PortError> {
    Ok(match code {
        SWATCH => Figure::Swatch(read_colour(reader)?),
        FILL => {
            let (corners, colour) = (read_points(reader)?, read_colour(reader)?);
            Figure::Fill { corners, colour }
        }
        PATCH => Figure::Patch(read_colour(reader)?),
        CELL => {
            let cell = usize::try_from(reader.u64()?).map_err(|_| reader.corrupt())?;
            let rect = [read_point(reader)?, read_point(reader)?];
            Figure::Cell { cell, rect }
        }
        PICTURE => {
            let (source, place) = (coded(reader, &SOURCES)?, read_place(reader)?);
            Figure::Picture {
                source,
                place,
                shown: reader.f32()?,
            }
        }
        PICTURED => Figure::Pictured,
        TENDENCY => Figure::Tendency(read_colour(reader)?),
        _ => return Err(reader.corrupt()),
    })
}

/// A laid picture's source (u8), place (a u8, 0 the frame, 1 between four corners that follow)
/// and strength (f32).
fn write_picture(writer: &mut Writer, (source, place, shown): (Source, &Place, f32)) {
    writer.u8(match source {
        Source::Pack => 0,
        Source::Painted => 1,
        Source::PaintedOrPack => 2,
        Source::GreyOriginal => 3,
        Source::GreyCopy => 4,
    });
    match place {
        Place::Frame => writer.u8(0),
        Place::Corners(corners) => {
            writer.u8(1);
            corners
                .iter()
                .for_each(|corner| write_point(writer, *corner));
        }
    }
    writer.f32(shown);
}

fn read_place(reader: &mut Reader<'_>) -> Result<Place, PortError> {
    match reader.u8()? {
        0 => Ok(Place::Frame),
        1 => {
            let mut corners = [[0.0; 2]; 4];
            for corner in &mut corners {
                *corner = read_point(reader)?;
            }
            Ok(Place::Corners(corners))
        }
        _ => Err(reader.corrupt()),
    }
}

fn write_pen(writer: &mut Writer, pen: LinePen) {
    writer.u8(match pen {
        LinePen::Ink => 0,
        LinePen::Printed => 1,
        LinePen::PrintedHalf => 2,
        LinePen::Answer => 3,
        LinePen::Laid => 4,
        LinePen::Off => 5,
        LinePen::Faint => 6,
        LinePen::Pencil => 7,
        LinePen::Level(Level::Main) => 8,
        LinePen::Level(Level::Second) => 9,
        LinePen::Level(Level::Third) => 10,
        LinePen::Hatch(Value::Dark) => 11,
        LinePen::Hatch(Value::Middle) => 12,
        LinePen::Hatch(Value::Light) => 13,
    });
}

fn read_pen(reader: &mut Reader<'_>) -> Result<LinePen, PortError> {
    coded(reader, &PENS)
}

/// The value whose code (a u8) is its place in `table`.
fn coded<T: Copy>(reader: &mut Reader<'_>, table: &[T]) -> Result<T, PortError> {
    let code = reader.u8()?;
    table
        .get(usize::from(code))
        .copied()
        .ok_or(reader.corrupt())
}

fn write_point(writer: &mut Writer, [x, y]: Point) {
    writer.f64(x);
    writer.f64(y);
}

fn read_point(reader: &mut Reader<'_>) -> Result<Point, PortError> {
    Ok([reader.f64()?, reader.f64()?])
}

fn write_points(writer: &mut Writer, points: &[Point]) {
    writer.len(points.len());
    points.iter().for_each(|point| write_point(writer, *point));
}

fn read_points(reader: &mut Reader<'_>) -> Result<Vec<Point>, PortError> {
    let count = reader.count(POINT_BYTES)?;
    (0..count).map(|_| read_point(reader)).collect()
}

fn read_colour(reader: &mut Reader<'_>) -> Result<Rgb8, PortError> {
    Ok([reader.u8()?, reader.u8()?, reader.u8()?])
}
