//! Typed SIDC modification: status, identity, affiliation and context.

use milsymbol::Renderer;
use milsymbol::domain::{Affiliation, Context, StandardIdentity, Status};
use milsymbol::sidc::{Sidc, SidcModifyError};
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const STATUSES: [Status; 6] = [
    Status::Present,
    Status::Planned,
    Status::FullyCapable,
    Status::Damaged,
    Status::Destroyed,
    Status::FullToCapacity,
];

const IDENTITIES: [StandardIdentity; 10] = [
    StandardIdentity::Pending,
    StandardIdentity::Unknown,
    StandardIdentity::AssumedFriend,
    StandardIdentity::Friend,
    StandardIdentity::Neutral,
    StandardIdentity::Suspect,
    StandardIdentity::Hostile,
    StandardIdentity::Joker,
    StandardIdentity::Faker,
    StandardIdentity::NoneSpecified,
];

const AFFILIATIONS: [Affiliation; 4] = [
    Affiliation::Friend,
    Affiliation::Hostile,
    Affiliation::Neutral,
    Affiliation::Unknown,
];

const NUMERIC: [&str; 3] = [
    "10031000001211000000",
    "10131000001211000000",
    "10231000001211000000",
];
const LETTER: [&str; 3] = ["SFGPUCI----D", "SDGPUCI----D", "OFVP------"];

fn identity_of(s: &Sidc) -> StandardIdentity {
    match s {
        Sidc::Numeric(n) => n.standard_identity(),
        Sidc::Letter(l) => l.standard_identity(),
    }
}

fn status_of(s: &Sidc) -> Status {
    match s {
        Sidc::Numeric(n) => n.status(),
        Sidc::Letter(l) => l.status(),
    }
}

fn context_of(s: &Sidc) -> Context {
    match s {
        Sidc::Numeric(n) => n.context(),
        Sidc::Letter(l) => l.context(),
    }
}

/// Positions (1-based) at which two SIDCs of equal length differ.
fn diff(a: &Sidc, b: &Sidc) -> Vec<usize> {
    assert_eq!(a.as_str().len(), b.as_str().len());
    a.as_str()
        .chars()
        .zip(b.as_str().chars())
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, _)| i + 1)
        .collect()
}

#[test]
fn status_round_trips_and_touches_only_the_status_position() -> TestResult {
    for text in NUMERIC.iter().chain(&LETTER) {
        let sidc = Sidc::parse(text)?;
        let pos = if matches!(sidc, Sidc::Numeric(_)) {
            7
        } else {
            4
        };
        for status in STATUSES {
            let changed = sidc.with_status(status);
            assert_eq!(status_of(&changed), status, "{text} {status:?}");
            assert!(diff(&sidc, &changed).iter().all(|&p| p == pos), "{text}");
            assert_eq!(Sidc::parse(changed.as_str())?, changed);
        }
    }
    Ok(())
}

#[test]
fn unchanged_letter_status_is_kept_verbatim() -> TestResult {
    let dash = Sidc::parse("SFG-UCI----D")?;
    assert_eq!(status_of(&dash), Status::Present);
    assert_eq!(dash.with_status(Status::Present), dash);
    assert_eq!(dash.with_status(Status::Damaged).as_str(), "SFGDUCI----D");
    assert_eq!(
        dash.with_status(Status::Damaged)
            .with_status(Status::Present)
            .as_str(),
        "SFGPUCI----D"
    );
    Ok(())
}

#[test]
fn identity_is_set_exactly_or_refused() -> TestResult {
    for text in NUMERIC.iter().chain(&LETTER) {
        let sidc = Sidc::parse(text)?;
        let context = context_of(&sidc);
        for identity in IDENTITIES {
            match sidc.with_standard_identity(identity) {
                Ok(changed) => {
                    assert_eq!(identity_of(&changed), identity, "{text} {identity:?}");
                    assert_eq!(context_of(&changed), context, "{text} {identity:?}");
                    assert_eq!(Sidc::parse(changed.as_str())?, changed);
                }
                Err(SidcModifyError::Unrepresentable {
                    identity: i,
                    context: c,
                }) => {
                    assert_eq!((i, c), (identity, context));
                    let exercise = context == Context::Exercise;
                    let needs_exercise =
                        matches!(identity, StandardIdentity::Joker | StandardIdentity::Faker);
                    let reality_only = matches!(
                        identity,
                        StandardIdentity::Suspect | StandardIdentity::Hostile
                    );
                    let none = identity == StandardIdentity::NoneSpecified;
                    assert!(
                        (needs_exercise && !exercise)
                            || (reality_only && exercise)
                            || (none && (exercise || matches!(sidc, Sidc::Numeric(_)))),
                        "{text} {identity:?} refused"
                    );
                }
                Err(other) => return Err(other.into()),
            }
        }
    }
    Ok(())
}

#[test]
fn affiliation_uses_the_identity_code_of_the_context() -> TestResult {
    let reality = Sidc::parse(NUMERIC[0])?;
    let exercise = Sidc::parse(NUMERIC[1])?;
    assert_eq!(
        reality.with_affiliation(Affiliation::Hostile).as_str(),
        "10061000001211000000"
    );
    assert_eq!(
        exercise.with_affiliation(Affiliation::Hostile).as_str(),
        "10161000001211000000"
    );
    assert_eq!(
        identity_of(&exercise.with_affiliation(Affiliation::Hostile)),
        StandardIdentity::Faker
    );
    let letter = Sidc::parse(LETTER[1])?;
    assert_eq!(
        letter.with_affiliation(Affiliation::Hostile).as_str(),
        "SKGPUCI----D"
    );
    assert_eq!(
        letter.with_affiliation(Affiliation::Neutral).as_str(),
        "SLGPUCI----D"
    );
    Ok(())
}

#[test]
fn rendered_frame_follows_the_modified_affiliation_and_status() -> TestResult {
    let renderer = Renderer::default();
    for text in [NUMERIC[0], LETTER[0]] {
        let sidc = Sidc::parse(text)?;
        for a in AFFILIATIONS {
            let symbol = renderer.symbol(&sidc.with_affiliation(a)).render()?;
            assert_eq!(symbol.metadata().affiliation, Some(a), "{text} {a:?}");
        }
        let planned = renderer
            .symbol(&sidc.with_status(Status::Planned))
            .render()?;
        assert!(planned.metadata().not_present, "{text}");
    }
    Ok(())
}

#[test]
fn context_keeps_the_meaning_of_the_identity() -> TestResult {
    for pair in [(NUMERIC[0], NUMERIC[1]), (LETTER[0], LETTER[1])] {
        let (reality, exercise) = (Sidc::parse(pair.0)?, Sidc::parse(pair.1)?);
        assert_eq!(reality.with_context(Context::Exercise)?, exercise);
        assert_eq!(exercise.with_context(Context::Reality)?, reality);
    }
    for (reality, exercise) in [
        ("SHGPUCI----D", "SKGPUCI----D"),
        ("SSGPUCI----D", "SJGPUCI----D"),
    ] {
        let r = Sidc::parse(reality)?;
        assert_eq!(r.with_context(Context::Exercise)?.as_str(), exercise);
        assert_eq!(Sidc::parse(exercise)?.with_context(Context::Reality)?, r);
    }
    let numeric = Sidc::parse(NUMERIC[0])?;
    assert_eq!(
        context_of(&numeric.with_context(Context::Simulation)?),
        Context::Simulation
    );
    Ok(())
}

#[test]
fn letter_contexts_without_a_code_are_refused() -> TestResult {
    let letter = Sidc::parse(LETTER[0])?;
    assert!(matches!(
        letter.with_context(Context::Simulation),
        Err(SidcModifyError::UnsupportedContext {
            context: Context::Simulation
        })
    ));
    let none = Sidc::parse("OOVP------")?;
    assert!(matches!(
        none.with_context(Context::Exercise),
        Err(SidcModifyError::Unrepresentable {
            identity: StandardIdentity::NoneSpecified,
            ..
        })
    ));
    Ok(())
}

fn numeric_sidc() -> impl Strategy<Value = String> {
    (0u8..3, 0u8..7, 0u8..6, 0u8..8)
        .prop_map(|(ctx, id, status, hq)| format!("10{ctx}{id}10{status}{hq}001211000000"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn numeric_modifications_stay_valid_and_exact(text in numeric_sidc()) {
        let sidc = Sidc::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        for status in STATUSES {
            let c = sidc.with_status(status);
            prop_assert_eq!(status_of(&c), status);
            prop_assert_eq!(&Sidc::parse(c.as_str()).map_err(|e| TestCaseError::fail(e.to_string()))?, &c);
        }
        for a in AFFILIATIONS {
            let c = sidc.with_affiliation(a);
            prop_assert_eq!(context_of(&c), context_of(&sidc));
            prop_assert!(diff(&sidc, &c).iter().all(|&p| p == 4));
        }
        for context in [Context::Reality, Context::Exercise, Context::Simulation] {
            let c = sidc.with_context(context).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(context_of(&c), context);
            prop_assert!(diff(&sidc, &c).iter().all(|&p| p == 3));
        }
    }
}

/// Every status, identity and context digit of 20- and 30-digit SIDCs, and
/// every identity and status letter of letter SIDCs.
fn all_sidcs() -> Result<Vec<Sidc>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for version in ["10", "13"] {
        for ctx in 0..3 {
            for id in 0..7 {
                for status in 0..6 {
                    let head = format!("{version}{ctx}{id}10{status}000121100");
                    out.push(Sidc::parse(&format!("{head}0000"))?);
                    out.push(Sidc::parse(&format!("{head}00000000000000"))?);
                }
            }
        }
    }
    for scheme in ["S", "G", "I", "O", "E"] {
        let dim = match scheme {
            "S" => "G",
            "G" => "G",
            "I" => "G",
            "O" => "G",
            _ => "N",
        };
        for id in "PUAFNSHGWMDLJKO".chars() {
            for status in "PACDXF-".chars() {
                out.push(Sidc::parse(&format!("{scheme}{id}{dim}{status}UCI----D"))?);
            }
        }
    }
    Ok(out)
}

#[test]
fn every_setter_on_every_identity_context_and_status_is_exact() -> TestResult {
    for sidc in all_sidcs()? {
        let numeric = matches!(sidc, Sidc::Numeric(_));
        let (id_pos, status_pos, ctx_pos) = if numeric { (4, 7, 3) } else { (2, 4, 2) };
        for status in STATUSES {
            let c = sidc.with_status(status);
            assert_eq!(status_of(&c), status, "{sidc}");
            assert!(diff(&sidc, &c).iter().all(|&p| p == status_pos), "{sidc}");
            assert_eq!(Sidc::parse(c.as_str())?, c);
        }
        for identity in IDENTITIES {
            if let Ok(c) = sidc.with_standard_identity(identity) {
                assert_eq!(identity_of(&c), identity, "{sidc} {identity:?}");
                assert_eq!(context_of(&c), context_of(&sidc), "{sidc}");
                assert!(diff(&sidc, &c).iter().all(|&p| p == id_pos), "{sidc}");
                assert_eq!(Sidc::parse(c.as_str())?, c);
            }
        }
        for a in AFFILIATIONS {
            let c = sidc.with_affiliation(a);
            assert_eq!(context_of(&c), context_of(&sidc), "{sidc} {a:?}");
            assert!(diff(&sidc, &c).iter().all(|&p| p == id_pos), "{sidc}");
            assert_eq!(Sidc::parse(c.as_str())?, c);
        }
        for context in [Context::Reality, Context::Exercise, Context::Simulation] {
            let Ok(c) = sidc.with_context(context) else {
                continue;
            };
            assert_eq!(context_of(&c), context, "{sidc} {context:?}");
            let allowed = |p: &usize| *p == ctx_pos || *p == id_pos;
            assert!(diff(&sidc, &c).iter().all(allowed), "{sidc} {context:?}");
            assert_eq!(Sidc::parse(c.as_str())?, c);
            let back = c.with_context(context_of(&sidc))?;
            assert_eq!(back, sidc, "{sidc} via {context:?}");
        }
    }
    Ok(())
}
