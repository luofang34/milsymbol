use super::*;

#[test]
fn typed_lookup_borrows_the_selected_slot() {
    let mut mode = ColorMode::new("pink", "blue", "red", "green", "yellow", "orange");
    for affiliation in [
        Affiliation::Friend,
        Affiliation::Hostile,
        Affiliation::Neutral,
        Affiliation::Unknown,
    ] {
        let selected = mode.for_affiliation(affiliation);
        assert_eq!(selected.cloned(), mode.get(affiliation.as_str()));
        let direct = mode.slot(affiliation.as_str()).and_then(Option::as_ref);
        assert!(
            selected
                .zip(direct)
                .is_some_and(|(a, b)| core::ptr::eq(a, b))
        );
    }
    mode.friend = None;
    mode.hostile = Some(Paint::None);
    assert_eq!(mode.for_affiliation(Affiliation::Friend), None);
    assert_eq!(
        mode.for_affiliation(Affiliation::Hostile),
        Some(&Paint::None)
    );
}
