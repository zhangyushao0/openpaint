//! Light on form's scene (S153): a solid of unit 3 (a box, a cylinder upright or lying, a cone on
//! its base, a sphere, sized as `solids/solid.rs` sizes them) or, at 稳, a carved object of unit 4
//! (`objects/object_design.rs` as it stands), standing on the ground, the eye a little above it
//! looking level (two-point, as the boxes), the horizon above the frame so the ground fills it; lit
//! by a sun 30 to 55 degrees high from where the variant says (three-quarter front, the side,
//! behind the form), given by a pin standing on the ground beside it (`geometry/light.rs`).
//!
//! A scene is laid again from its seed's sequence until it keeps every rule: the form whole in the
//! frame; some of each family seen on it; its cast shadow inside the frame and wide enough to
//! paint; the pin and its shadow clear of the form and its shadow; at 熟练 and 稳 a round form's
//! terminator crossing its visible side; the horizon above the frame. Rules that need the light
//! are read off a coarse survey of the model (`survey.rs`).

use crate::geometry::light::{Light, Pin};
use crate::geometry::perspective_eye::{box_corners, directions, framing, projected};
use crate::kinds::light_on_form::outline::{contact, creases, printed};
use crate::kinds::light_on_form::survey::Survey;
use crate::kinds::objects::object_body::{Ball, Body, Placed, Scene};
use crate::kinds::objects::object_design::{Block, Carve, Design, Turned, designed};
use crate::kinds::objects::object_hull::round;
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::maths::vector::{Vector, times};

/// Scenes laid before the generator keeps one whatever it breaks; pins tried a scene.
const TRIES: usize = 300;
const PIN_TRIES: usize = 24;
/// The sun's height above the ground, degrees.
const ELEVATION: [f64; 2] = [30.0, 55.0];
/// The form's box's longer side in the frame; the eye above the form's middle beyond its breadth
/// (a share of it) and the form's distance.
const SIDE: [f64; 2] = [0.34, 0.42];
const EYE: [f64; 2] = [0.1, 0.6];
const EYE_OVER: f64 = 2.0;
const DISTANCE: [f64; 2] = [6.0, 8.0];
/// How far the form lies from the frame's middle against its shadow's way.
const AGAINST: f64 = 0.1;
/// The form's box inside the frame by this.
const INSIDE: f64 = 0.06;
/// The horizon at least this far above the frame's top.
const HORIZON: f64 = -0.02;

/// What stands in the light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    Box,
    Cylinder,
    Cone,
    Sphere,
    /// A carved object of unit 4 (稳).
    Object,
}

/// Where the sun stands against the eye: over the shoulder (three-quarter front), at the side, or
/// beyond the form (edge light).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lighting {
    Front,
    Side,
    Behind,
}

impl Lighting {
    /// The sun's way along the ground from the way back to the eye, degrees (`Light::sun`).
    const fn azimuth(self) -> [f64; 2] {
        match self {
            Self::Front => [40.0, 65.0],
            Self::Side => [80.0, 100.0],
            Self::Behind => [120.0, 150.0],
        }
    }
}

/// The greys a level paints with: the light and the dark (入门); the middle too (熟练); the
/// darkest too (稳), the core, the reflected light and the contact told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tones {
    Two,
    Three,
    Four,
}

/// Light on form's variant: the form, where the sun stands and the level's greys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit {
    pub form: Form,
    pub lighting: Lighting,
    pub tones: Tones,
}

impl Lit {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 15] = [
        Self::of(Form::Box, Lighting::Front, Tones::Two),
        Self::of(Form::Cylinder, Lighting::Front, Tones::Two),
        Self::of(Form::Sphere, Lighting::Front, Tones::Two),
        Self::of(Form::Box, Lighting::Side, Tones::Three),
        Self::of(Form::Cylinder, Lighting::Side, Tones::Three),
        Self::of(Form::Cone, Lighting::Side, Tones::Three),
        Self::of(Form::Sphere, Lighting::Side, Tones::Three),
        Self::of(Form::Box, Lighting::Behind, Tones::Three),
        Self::of(Form::Cylinder, Lighting::Behind, Tones::Three),
        Self::of(Form::Cone, Lighting::Behind, Tones::Three),
        Self::of(Form::Sphere, Lighting::Behind, Tones::Three),
        Self::of(Form::Cylinder, Lighting::Front, Tones::Four),
        Self::of(Form::Cone, Lighting::Front, Tones::Four),
        Self::of(Form::Sphere, Lighting::Front, Tones::Four),
        Self::of(Form::Object, Lighting::Front, Tones::Four),
    ];

    const fn of(form: Form, lighting: Lighting, tones: Tones) -> Self {
        Self {
            form,
            lighting,
            tones,
        }
    }
}

/// A form in the light to paint in its families, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct LightTask {
    pub lit: Lit,
    pub(crate) scene: Scene,
    pub(crate) view: View,
    pub(crate) light: Light,
    pub(crate) pin: Pin,
    /// The box holding the form, in the world (`PerspectiveBox`'s corner order).
    pub(crate) corners: [Vector; 8],
    /// The form's visible edges and outlines, printed (frame units).
    pub edges: Vec<Vec<Point>>,
    /// The pin's foot, top and shadow's tip (frame units).
    pub pinned: [Point; 3],
    /// Where the form meets the ground (frame units): lines, or a point alone.
    pub(crate) contact: Vec<Vec<Point>>,
    /// The visible edges between a lit face and an unlit one (frame units).
    pub(crate) creases: Vec<[Point; 2]>,
}

impl LightTask {
    /// The task `lit` and `seed` make.
    #[must_use]
    pub fn generate(lit: Lit, seed: u64) -> Self {
        let mut random = Random::new(seed);
        (0..TRIES)
            .find_map(|_| laid(lit, &mut random))
            .unwrap_or_else(|| plain(lit))
    }

    /// The way from the eye through the frame point `p`.
    pub(crate) fn ray(&self, p: Point) -> Vector {
        let View { scale, offset } = self.view;
        [(p[0] - offset[0]) / scale, -(p[1] - offset[1]) / scale, 1.0]
    }

    /// The ground's point the eye sees at the frame point `p`; none above the horizon.
    pub(crate) fn on_ground(&self, p: Point) -> Option<Vector> {
        let d = self.ray(p);
        (d[1] < 0.0).then(|| times(d, self.scene.ground / d[1]))
    }

    /// The horizon's height in the frame (above it: negative).
    #[must_use]
    pub fn horizon(&self) -> f64 {
        self.view.offset[1]
    }
}

/// The generator's last resort (never met by a seed tested): a sphere in a fixed light, its pin in
/// a fixed place.
fn plain(lit: Lit) -> LightTask {
    let drawn = Drawn {
        size: [1.0; 3],
        made: Made::Ball,
        yaw: 0.7,
        ground: -2.0,
        middle: [0.0, 3.8],
        light: Light::sun(0.75, 0.8),
        side: 0.38,
        jitter: [0.0; 2],
    };
    let mut task = built(lit, drawn);
    let foot = task.on_ground([0.18, 0.85]).unwrap_or(task.corners[0]);
    task.stand(Pin::at(foot, 0.5));
    task
}

/// How the form is made in its box.
enum Made {
    Box,
    Turned(Turned),
    Ball,
    Design(Design),
}

/// The form `form` laid from `random`: its box's width, height and depth, and how it is made.
fn shaped(form: Form, random: &mut Random) -> Option<([f64; 3], Made)> {
    let turned = |axis: usize, cone: bool| {
        Made::Turned(Turned {
            block: Block::WHOLE,
            axis,
            cone,
        })
    };
    Some(match form {
        Form::Box => ([0; 3].map(|_| random.range(0.6, 1.3)), Made::Box),
        Form::Cylinder => {
            let square = random.range(0.7, 1.1);
            let long = square * random.range(1.1, 1.8);
            match random.unit() {
                roll if roll < 0.6 => ([square, long, square], turned(1, false)),
                roll if roll < 0.8 => ([long, square, square], turned(0, false)),
                _ => ([square, square, long], turned(2, false)),
            }
        }
        Form::Cone => {
            let square = random.range(0.8, 1.2);
            (
                [square, square * random.range(1.0, 1.6), square],
                turned(1, true),
            )
        }
        Form::Sphere => ([random.range(0.8, 1.2); 3], Made::Ball),
        Form::Object => {
            let carve = Carve::ALL[(random.next_u64() % 4) as usize];
            let design = designed(carve, random, false)?;
            (design.size, Made::Design(design))
        }
    })
}

/// The body `made` makes in the box `placed` of `size`.
fn body(made: &Made, size: [f64; 3], placed: Placed) -> Body {
    let bare = |rounds, balls| Body {
        placed,
        hulls: Vec::new(),
        rounds,
        balls,
    };
    match made {
        Made::Box => Body::of(
            &Design {
                changes: Vec::new(),
                size,
            },
            placed,
        ),
        Made::Turned(turned) => bare(vec![round(turned, placed)], Vec::new()),
        Made::Ball => bare(
            Vec::new(),
            vec![Ball {
                centre: placed.world([0.5; 3]),
                radius: size[0] / 2.0,
            }],
        ),
        Made::Design(design) => Body::of(design, placed),
    }
}

/// What a scene is laid from: the form's box's size and how the form is made in it, the box's
/// turn, the ground's height, the box's middle across and away from the eye, the light, the box's
/// longer side in the frame and how far off its place against its shadow it lies.
struct Drawn {
    size: [f64; 3],
    made: Made,
    yaw: f64,
    ground: f64,
    middle: [f64; 2],
    light: Light,
    side: f64,
    jitter: Point,
}

/// What a scene of `lit` is laid from, drawn from `random`; none where its form cannot be made.
fn drawn(lit: Lit, random: &mut Random) -> Option<Drawn> {
    let (size, made) = shaped(lit.form, random)?;
    let yaw = random.range(25.0, 65.0).to_radians();
    // High enough above the form's middle, for its height and its breadth, that the horizon
    // lies above the frame whatever its shape.
    let breadth = size[1].max(size[0].hypot(size[2]));
    let ground = -(size[1] / 2.0 + EYE_OVER * breadth + random.range(EYE[0], EYE[1]));
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    let middle = [random.range(-0.15, 0.15) * depth, depth];
    let [low, high] = lit.lighting.azimuth();
    let azimuth = random.sign() * random.range(low, high).to_radians();
    let elevation = random.range(ELEVATION[0], ELEVATION[1]).to_radians();
    Some(Drawn {
        size,
        made,
        yaw,
        ground,
        middle,
        light: Light::sun(elevation, azimuth),
        side: random.range(SIDE[0], SIDE[1]),
        jitter: [random.range(-0.04, 0.04), random.range(-0.03, 0.03)],
    })
}

/// The scene of `lit` `drawn` lays, its pin not yet placed.
fn built(lit: Lit, drawn: Drawn) -> LightTask {
    let Drawn { size, ground, .. } = drawn;
    let [across, depth] = drawn.middle;
    let middle = [across, ground + size[1] / 2.0, depth];
    let corners = box_corners(middle, drawn.yaw, size);
    let [left, right, up] = directions(drawn.yaw);
    let placed = Placed {
        origin: corners[0],
        axes: [
            times(right, size[0]),
            times(up, size[1]),
            times(left, size[2]),
        ],
    };
    // The form against its shadow's way, so the shadow has room.
    let foot = [across, ground, depth];
    let top = drawn
        .light
        .shadow([across, ground + size[1], depth], ground);
    let [from, to] = [foot, top.unwrap_or(foot)].map(projected);
    let way = [to[0] - from[0], to[1] - from[1]];
    let long = way[0].hypot(way[1]).max(f64::MIN_POSITIVE);
    let shift = [0, 1].map(|k| -way[k] / long * AGAINST + drawn.jitter[k]);
    let image = corners.map(projected);
    let (scale, offset) = framing(&image, 0.0, (drawn.side, shift), None);
    let view = View { scale, offset };
    let scene = Scene {
        body: body(&drawn.made, size, placed),
        ground,
    };
    LightTask {
        lit,
        contact: contact(&scene, view),
        creases: creases(&scene, &drawn.light, view),
        scene,
        view,
        light: drawn.light,
        pin: Pin::at(corners[0], 0.0),
        corners,
        edges: Vec::new(),
        pinned: [[0.0; 2]; 3],
    }
}

/// One scene of `lit` laid from `random`; none when it breaks a rule.
fn laid(lit: Lit, random: &mut Random) -> Option<LightTask> {
    let mut task = built(lit, drawn(lit, random)?);
    let survey = Survey::of(&task);
    if !(task.inside() && survey.keeps(&task)) {
        return None;
    }
    let pin = (0..PIN_TRIES).find_map(|_| survey.pin(&task, random))?;
    task.stand(pin);
    Some(task)
}

impl LightTask {
    /// Whether the form's box lies in the frame, and the horizon above it.
    pub(crate) fn inside(&self) -> bool {
        let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
        let corners = self.corners.map(|c| self.view.at(c));
        corners.iter().all(|c| inside(c[0]) && inside(c[1])) && self.horizon() <= HORIZON
    }

    /// The scene with `pin` standing in it, and what it prints.
    fn stand(&mut self, pin: Pin) {
        let tip = pin.tip(&self.light).unwrap_or(pin.foot);
        self.pinned = [pin.foot, pin.top, tip].map(|p| self.view.at(p));
        self.pin = pin;
        self.edges = printed(self);
    }
}
