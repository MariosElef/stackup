/// An enum the user picks from: clap flag value, list of all variants, and display label.
macro_rules! choice {
    ($name:ident { $($var:ident => $label:expr),+ $(,)? }) => {
        #[derive(Clone, Copy, PartialEq, Eq, ::clap::ValueEnum, Debug)]
        pub enum $name { $($var),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var),+];
        }
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(match self { $($name::$var => $label),+ })
            }
        }
    };
}

pub(crate) use choice;
