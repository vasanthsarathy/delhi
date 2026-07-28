//! The examples that ship inside the binary.
//!
//! A downloaded `delhi` has no repository beside it, so without these the UI opens on an
//! empty directory and a new user has nowhere to learn what a `.delhi` file looks like.
//! They are read-only: the copy in the binary is the one that is served, and saving one
//! writes a new file into the served directory under whatever name the user chooses.
//!
//! `lib.rs` has a test asserting this list matches `examples/` exactly, because a missing
//! entry is invisible from inside the repository — the directory is right there.

/// Bundled examples as `(file name, source)`, sorted by name.
pub const BUILTIN: &[(&str, &str)] = &[
    ("bicycle.delhi", include_str!("../../../examples/bicycle.delhi")),
    ("coin_in_the_box.delhi", include_str!("../../../examples/coin_in_the_box.delhi")),
    ("coin_lie.delhi", include_str!("../../../examples/coin_lie.delhi")),
    ("grapevine.delhi", include_str!("../../../examples/grapevine.delhi")),
    ("ice_cream_van.delhi", include_str!("../../../examples/ice_cream_van.delhi")),
    ("muddy_children.delhi", include_str!("../../../examples/muddy_children.delhi")),
    ("reachability.delhi", include_str!("../../../examples/reachability.delhi")),
    ("sally_anne.delhi", include_str!("../../../examples/sally_anne.delhi")),
    (
        "sally_anne_second_order.delhi",
        include_str!("../../../examples/sally_anne_second_order.delhi"),
    ),
    (
        "selective_communication.delhi",
        include_str!("../../../examples/selective_communication.delhi"),
    ),
];
