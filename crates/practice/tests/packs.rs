//! A pack's manifest: codes and names that never change, the rules every pack keeps, and how an
//! exercise finds its picture.

use openpaint_practice::{
    Credit, ItemRef, Licence, MAX_PICTURE_SIDE, Outline, PERSONAL, Pack, PackItem, PackProblem,
    PictureKind, Tier, valid_id,
};

fn item(id: &str, tier: Tier, licence: Licence) -> PackItem {
    PackItem {
        id: id.to_owned(),
        kind: PictureKind::Shape,
        tier,
        size: [1200, 900],
        title: "Teapot".to_owned(),
        credit: Some(Credit {
            source: format!("https://example.org/{id}"),
            creator: "Unknown maker".to_owned(),
            licence,
            attribution: String::new(),
            notes: "one handle, one spout".to_owned(),
        }),
        outline: None,
    }
}

fn credit(pack: &mut Pack, at: usize) -> &mut Credit {
    pack.items[at].credit.as_mut().expect("credited")
}

fn pack() -> Pack {
    Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: vec![
            item("teapot", Tier::Simple, Licence::Cc0),
            item("fox", Tier::Entry, Licence::Cc0),
            item("helmet", Tier::Hard, Licence::PdOld),
            item("vase", Tier::Simple, Licence::Cc0),
        ],
    }
}

#[test]
fn codes_and_names_are_fixed_and_round_trip() {
    let codes: Vec<(u8, &str)> = (Licence::ALL.iter())
        .map(|licence| (licence.code(), licence.name()))
        .collect();
    assert_eq!(
        codes,
        [
            (1, "CC0"),
            (2, "PD-US-gov"),
            (3, "PD-old"),
            (4, "CC BY 4.0"),
            (5, "CC BY 3.0"),
            (6, "CC BY 2.5"),
            (7, "CC BY 2.0"),
        ]
    );
    for licence in Licence::ALL {
        assert_eq!(Licence::of_code(licence.code()), Some(licence));
        assert_eq!(Licence::of_name(licence.name()), Some(licence));
        assert_eq!(
            licence.requires_attribution(),
            licence.name().starts_with("CC BY")
        );
    }
    let tiers: Vec<(u8, &str)> = Tier::ALL.iter().map(|t| (t.code(), t.name())).collect();
    assert_eq!(
        tiers,
        [(1, "entry"), (2, "simple"), (3, "medium"), (4, "hard")]
    );
    for tier in Tier::ALL {
        assert_eq!(Tier::of_code(tier.code()), Some(tier));
        assert_eq!(Tier::of_name(tier.name()), Some(tier));
    }
    let kinds: Vec<(u8, &str)> = (PictureKind::ALL.iter())
        .map(|kind| (kind.code(), kind.name()))
        .collect();
    assert_eq!(kinds, [(1, "shape"), (2, "composition")]);
    for kind in PictureKind::ALL {
        assert_eq!(PictureKind::of_code(kind.code()), Some(kind));
        assert_eq!(PictureKind::of_name(kind.name()), Some(kind));
    }
    assert_eq!(PictureKind::of_code(3), None);
    assert_eq!(Licence::of_code(0), None);
    assert_eq!(
        Licence::of_name("CC BY-SA 4.0"),
        None,
        "share-alike is no pack's"
    );
    assert_eq!(Tier::of_code(5), None);
}

#[test]
fn ids_are_file_names() {
    for id in ["openpaint-shapes", "cma-1940-228", "a", &"x".repeat(64)] {
        assert!(valid_id(id), "{id}");
    }
    for id in [
        "",
        "-teapot",
        "Teapot",
        "tea pot",
        "tea/pot",
        "teapot.png",
        &"x".repeat(65),
    ] {
        assert!(!valid_id(id), "{id}");
    }
}

fn broken(change: impl FnOnce(&mut Pack)) -> Result<(), PackProblem> {
    let mut pack = pack();
    change(&mut pack);
    pack.check()
}

#[test]
fn every_rule_is_checked() {
    assert_eq!(pack().check(), Ok(()));
    let item = |rule| {
        Err(PackProblem {
            item: Some("vase".to_owned()),
            rule,
        })
    };
    assert!(broken(|p| p.id = "Shapes".into()).is_err());
    assert!(broken(|p| p.title = " ".into()).is_err());
    assert!(broken(|p| p.items.clear()).is_err());
    let side = MAX_PICTURE_SIDE + 1;
    assert_eq!(
        broken(|p| p.items[3].size = [side, 10]),
        item("its picture's sides are not 1 to 2048 pixels")
    );
    assert!(broken(|p| p.items[3].size = [0, 10]).is_err());
    assert!(broken(|p| credit(p, 3).source = "http://example.org".into()).is_err());
    assert!(broken(|p| credit(p, 3).creator = String::new()).is_err());
    assert_eq!(
        broken(|p| p.items[3].id = "teapot".into()),
        Err(PackProblem {
            item: Some("teapot".to_owned()),
            rule: "its id is taken by an item before it",
        })
    );
    assert_eq!(
        broken(|p| credit(p, 3).licence = Licence::CcBy4),
        item("its licence asks for an attribution and it has none")
    );
    assert_eq!(
        broken(|p| {
            credit(p, 3).licence = Licence::CcBy4;
            credit(p, 3).attribution = "“Vase” by A. Potter, CC BY 4.0".into();
        }),
        Ok(())
    );
    let shown = broken(|p| p.items[3].size = [0, 0]).expect_err("broken");
    assert!(shown.to_string().starts_with("item vase: "), "{shown}");
    assert_eq!(
        broken(|p| p.items[3].credit = None),
        item("an official item has no credit")
    );
}

#[test]
fn an_outline_lies_inside_its_picture_and_closes() {
    let inside = Outline {
        contours: vec![vec![[10.0, 10.0], [1190.0, 10.0], [600.0, 890.0]]],
    };
    assert_eq!(
        broken(|p| p.items[3].outline = Some(inside.clone())),
        Ok(())
    );
    let mut outside = inside.clone();
    outside.contours[0][1] = [1201.0, 10.0];
    assert!(broken(|p| p.items[3].outline = Some(outside)).is_err());
    let open = Outline {
        contours: vec![vec![[10.0, 10.0], [20.0, 20.0]]],
    };
    assert!(broken(|p| p.items[3].outline = Some(open)).is_err());
    assert!(broken(|p| p.items[3].outline = Some(Outline { contours: vec![] })).is_err());
    assert_eq!(
        inside.lines()[0].len(),
        4,
        "a line closes on its first point"
    );
}

#[test]
fn a_composition_takes_three_tiers_and_never_an_outline() {
    let composition = |p: &mut Pack| p.items[3].kind = PictureKind::Composition;
    assert_eq!(broken(composition), Ok(()), "a simple composition");
    let item = |rule| {
        Err(PackProblem {
            item: Some("vase".to_owned()),
            rule,
        })
    };
    assert_eq!(
        broken(|p| {
            composition(p);
            p.items[3].tier = Tier::Entry;
        }),
        item("its tier is not one its kind takes")
    );
    let outline = Outline {
        contours: vec![vec![[10.0, 10.0], [1190.0, 10.0], [600.0, 890.0]]],
    };
    assert_eq!(
        broken(|p| {
            composition(p);
            p.items[3].outline = Some(outline.clone());
        }),
        item("its kind is compared without an outline and it has one")
    );
    assert_eq!(
        PictureKind::Composition.tiers(),
        [Tier::Simple, Tier::Medium, Tier::Hard]
    );
    assert_eq!(PictureKind::Shape.tiers(), Tier::ALL);
    assert!(PictureKind::Shape.outlined() && !PictureKind::Composition.outlined());
    let mut mixed = pack();
    composition(&mut mixed);
    let shapes: Vec<&str> = (mixed.of(PictureKind::Shape, Tier::Simple))
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(shapes, ["teapot"], "a composition is no shape");
}

#[test]
fn the_personal_pack_credits_nothing_and_shows_no_credit() {
    let mut mine = pack();
    mine.id = PERSONAL.to_owned();
    assert!(mine.personal());
    assert!(mine.check().is_err(), "a credited picture of your own");
    for item in &mut mine.items {
        item.credit = None;
    }
    assert_eq!(mine.check(), Ok(()));
    assert!(mine.items.iter().all(|item| item.shown_credit().is_none()));
    let mut official = pack();
    assert_eq!(official.items[0].shown_credit(), None, "CC0: none asked");
    credit(&mut official, 0).licence = Licence::CcBy4;
    credit(&mut official, 0).attribution = "“Teapot” by A. Potter, CC BY 4.0".into();
    assert_eq!(
        official.items[0].shown_credit(),
        Some("“Teapot” by A. Potter, CC BY 4.0")
    );
}

#[test]
fn an_exercise_finds_its_picture_by_pack_and_id() {
    let pack = pack();
    let named = |pack: &str, item: &str| ItemRef {
        pack: pack.to_owned(),
        item: item.to_owned(),
    };
    assert_eq!(
        pack.find(&named("test-shapes", "helmet")).map(|i| i.tier),
        Some(Tier::Hard)
    );
    assert_eq!(pack.find(&named("other-pack", "helmet")), None);
    assert_eq!(pack.find(&named("test-shapes", "lamp")), None);
    let simple: Vec<&str> = pack
        .of(PictureKind::Shape, Tier::Simple)
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(simple, ["teapot", "vase"]);
}
