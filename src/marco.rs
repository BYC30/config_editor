macro_rules! check_if {
    ($cond:expr, $else:expr) => {
        if $cond {
            $else;
        }
    };
}

macro_rules! check_some {
    ($cond:expr, $else:expr) => {
        match $cond {
            Some(s) => s,
            None => $else,
        }
    };
}

pub(crate) use check_if;
pub(crate) use check_some;
