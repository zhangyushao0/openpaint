use std::fs::File;

use super::*;

const NOW: &str = "aaaaaaaaaaaaaaaa";
const OLD: &str = "bbbbbbbbbbbbbbbb";
const CHECKED: &str = "cccccccccccccccc";
const OTHER: &str = "dddddddddddddddd";
const GONE: &str = "eeeeeeeeeeeeeeee";
const GONE_PACKAGE: &str = "ffffffffffffffff";
const LIB: &str = "1111111111111111";
const LIB_OLD: &str = "2222222222222222";
const LIB_YOUNG: &str = "3333333333333333";

/// A folder of its own under `target/tmp` for one test; kept when the test fails.
fn scratch(name: &str) -> PathBuf {
    let dir = root()
        .join("target")
        .join("tmp")
        .join(format!("xtask-prune-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("the scratch folder");
    dir
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("its folder");
    fs::write(path, b"x").expect("written");
}

/// A fingerprint folder for `target` built under `package-hash`, its timestamp `age` old.
fn fingerprint(profile: &Path, package: &str, hash: &str, target: &str, age: Duration) {
    let folder = profile
        .join(".fingerprint")
        .join(format!("{package}-{hash}"));
    for name in [target, &format!("{target}.json"), &format!("dep-{target}")] {
        touch(&folder.join(name));
    }
    let timestamp = folder.join("invoked.timestamp");
    touch(&timestamp);
    let file = File::options().write(true).open(&timestamp).expect("open");
    file.set_modified(SystemTime::now() - age)
        .expect("timestamp");
}

/// A profile folder holding builds of a test binary and of a library, and the units of the
/// build made now.
fn builds(profile: &Path) -> [Unit; 2] {
    let deps = profile.join("deps");
    let fill = "test-integration-test-fill";
    let young = Duration::ZERO;
    let old = STALE + Duration::from_secs(60);
    // The test binary built now, one linked before, clippy's check of it (no executable), and
    // a test of the same name in another package.
    for (hash, files) in [
        (NOW, &[EXE_SUFFIX, ".pdb", ".d"][..]),
        (OLD, &[EXE_SUFFIX, ".pdb", ".d"]),
        (CHECKED, &[".rmeta", ".d"]),
        (OTHER, &[EXE_SUFFIX, ".pdb"]),
    ] {
        for extension in files {
            touch(&deps.join(format!("fill-{hash}{extension}")));
        }
    }
    for hash in [NOW, OLD, CHECKED] {
        fingerprint(profile, "openpaint-app", hash, fill, young);
    }
    fingerprint(profile, "openpaint-raster", OTHER, fill, young);
    // A test the app no longer has, and one of a package not built now.
    for (package, hash) in [("openpaint-app", GONE), ("openpaint-gone", GONE_PACKAGE)] {
        touch(&deps.join(format!("autosave-{hash}{EXE_SUFFIX}")));
        fingerprint(
            profile,
            package,
            hash,
            "test-integration-test-autosave",
            young,
        );
    }
    // The library built now, one built two days ago, one checked today, and a dependency.
    let core = "lib-openpaint_core";
    for (hash, age) in [(LIB, young), (LIB_OLD, old), (LIB_YOUNG, young)] {
        for file in [".rlib", ".rmeta"].map(|ext| format!("libopenpaint_core-{hash}{ext}")) {
            touch(&deps.join(file));
        }
        touch(&deps.join(format!("openpaint_core-{hash}.d")));
        fingerprint(profile, "openpaint-core", hash, core, age);
    }
    touch(&deps.join(format!("libserde-{LIB_OLD}.rlib")));
    let unit = |stem: &str, hash: &str, test| Unit {
        deps: deps.clone(),
        stem: stem.to_owned(),
        hash: hash.to_owned(),
        test,
    };
    [
        unit("fill", NOW, true),
        unit("libopenpaint_core", LIB, false),
    ]
}

#[test]
fn a_build_takes_its_targets_other_builds_and_nothing_of_another_target() {
    let profile = scratch("builds").join("debug");
    let deps = profile.join("deps");
    let units = builds(&profile);
    let mut found = superseded(&units, SystemTime::now());
    found.sort();
    let fingerprints = profile.join(".fingerprint");
    let mut expected = vec![
        deps.join(format!("fill-{OLD}{EXE_SUFFIX}")),
        deps.join(format!("fill-{OLD}.pdb")),
        deps.join(format!("fill-{OLD}.d")),
        fingerprints.join(format!("openpaint-app-{OLD}")),
        deps.join(format!("autosave-{GONE}{EXE_SUFFIX}")),
        fingerprints.join(format!("openpaint-app-{GONE}")),
        deps.join(format!("libopenpaint_core-{LIB_OLD}.rlib")),
        deps.join(format!("libopenpaint_core-{LIB_OLD}.rmeta")),
        deps.join(format!("openpaint_core-{LIB_OLD}.d")),
        fingerprints.join(format!("openpaint-core-{LIB_OLD}")),
    ];
    expected.sort();
    assert_eq!(found, expected);
    let removed = remove(&found);
    assert_eq!((removed.files, removed.folders, removed.kept), (7, 3, 0));
    assert!(found.iter().all(|path| !path.exists()));
    assert!(
        superseded(&units, SystemTime::now()).is_empty(),
        "nothing left"
    );
    assert!(deps.join(format!("fill-{CHECKED}.rmeta")).is_file());
    fs::remove_dir_all(profile.parent().expect("the scratch folder")).expect("cleaned");
}

#[test]
fn incremental_caches_go_once_unwritten_for_a_day() {
    let profile = scratch("caches").join("debug");
    let cache = profile
        .join("incremental")
        .join("openpaint_core-27b35x2n0eb66");
    touch(&cache.join("s-session").join("query-cache.bin"));
    let missing = profile
        .with_file_name("aarch64-linux-android")
        .join("debug");
    let profiles = [profile.clone(), missing];
    assert!(stale_caches(&profiles, SystemTime::now()).is_empty());
    let tomorrow = SystemTime::now() + STALE + Duration::from_secs(60);
    assert_eq!(
        stale_caches(&profiles, tomorrow),
        std::slice::from_ref(&cache)
    );
    let removed = remove(std::slice::from_ref(&cache));
    assert_eq!((removed.files, removed.folders, removed.bytes), (0, 1, 1));
    assert!(!cache.exists());
    fs::remove_dir_all(profile.parent().expect("the scratch folder")).expect("cleaned");
}

#[test]
fn units_are_the_workspace_tests_and_libraries_cargo_announces() {
    let deps = root().join("target").join("debug").join("deps");
    let escaped = |path: PathBuf| path.display().to_string().replace('\\', "\\\\");
    let exe = escaped(deps.join(format!("fill-{NOW}{EXE_SUFFIX}")));
    let rlib = escaped(deps.join(format!("libopenpaint_core-{LIB}.rlib")));
    let rmeta = escaped(deps.join(format!("libopenpaint_core-{LIB}.rmeta")));
    let message = |package: &str, test: bool, filenames: &str, executable: &str| {
        format!(
            "{{\"reason\":\"compiler-artifact\",\"package_id\":\"{package}\",\
             \"profile\":{{\"opt_level\":\"0\",\"test\":{test}}},\"features\":[],\
             \"filenames\":[{filenames}],\"executable\":{executable},\"fresh\":true}}"
        )
    };
    let workspace = "path+file:///C:/repo/openpaint/crates/core#openpaint-core@1.21.1";
    let test = message(
        workspace,
        true,
        &format!("\"{exe}\""),
        &format!("\"{exe}\""),
    );
    let found = unit(&test).expect("a test binary");
    assert_eq!((found.stem.as_str(), found.hash.as_str()), ("fill", NOW));
    assert!(found.test && found.deps == deps);
    let library = message(workspace, false, &format!("\"{rlib}\",\"{rmeta}\""), "null");
    let found = unit(&library).expect("a library");
    assert_eq!(found.stem, "libopenpaint_core");
    assert_eq!((found.hash.as_str(), found.test), (LIB, false));
    let checked = message(workspace, false, &format!("\"{rmeta}\""), "null");
    assert!(unit(&checked).is_none(), "checked, not built");
    let registry = "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.229";
    assert!(unit(&message(registry, false, &format!("\"{rlib}\""), "null")).is_none());
    assert!(unit("{\"reason\":\"build-finished\",\"success\":true}").is_none());
    assert_eq!(
        split("fill-0123456789abcdef"),
        Some(("fill", "0123456789abcdef"))
    );
    assert_eq!(split("openpaint_harness"), None);
    assert_eq!(split("a-b-0123"), None);
}
