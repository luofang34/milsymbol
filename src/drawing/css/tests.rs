use super::is_color;

/// Strings milsymbol.js's colour modes produce, the corpus colours and a
/// spread of malformed ones, judged by svgtypes (the parser usvg and resvg
/// use) and by this module.
const SAMPLES: [&str; 45] = [
    "red",
    "RED",
    " red ",
    "\tpink\n",
    "rgb(128,224,255)",
    "rgb(239, 239, 239)",
    "rgb(1,2,3)",
    "rgba(255,255,255,0.4)",
    "rgb(10%, 20%, 30%)",
    "hsl(120, 100%, 75%)",
    "hsla(120, 100%, 75%, 0.5)",
    "#f00",
    "#ff0000",
    "#ff000080",
    "#f008",
    "transparent",
    "black",
    "OffWhite",
    "rbg(255, 188, 1)",
    "\u{85}red\u{85}",
    "\u{3000}pink\u{feff}",
    "",
    " ",
    "#ff",
    "#ff00000",
    "#gg0000",
    "rgb(1,2)",
    "rgb(1,2,3",
    "rgb 1,2,3)",
    "rgbx(1,2,3)",
    "url(#a)",
    "none",
    "javascript:alert(1)",
    "red blue",
    "rgb(1,2,3)x",
    "1e3",
    "#",
    "rgb(,1,2,3)",
    "hsl(a, 1%, 2%)",
    "rgb(1.5e1, 2, 3)",
    "rgb(.5,2,3)",
    "rgb(5.,2,3)",
    "redd",
    "currentColor",
    "rgb(1 2 3)",
];

#[test]
fn agrees_with_svgtypes() {
    for s in SAMPLES {
        let svgtypes = s.parse::<svgtypes::Color>().is_ok();
        assert_eq!(is_color(s), svgtypes, "{s:?}");
    }
}

/// CSS Color 4 accepts these and browsers draw them; svgtypes does not
/// know them. The drawing view follows CSS.
#[test]
fn accepts_css_color_4_forms_svgtypes_lacks() {
    for s in ["rebeccapurple", "rgb(1 2 3 / 50%)", "hsl(120deg 100% 50%)"] {
        assert!(is_color(s), "{s:?}");
        assert!(s.parse::<svgtypes::Color>().is_err(), "{s:?}");
    }
}

#[test]
fn rejects_mixed_separators() {
    for s in [
        "rgb(1, 2 3)",
        "rgb(1,2,3 / 0.5)",
        "rgb(1 2 3 4)",
        "rgb(1 2 / 3)",
    ] {
        assert!(!is_color(s), "{s:?}");
    }
}
