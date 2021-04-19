#[macro_export]
macro_rules! hlist {
    ($x:ty, $($tail:tt)+) => { ($x, hlist!($($tail)+)) };
    ($x:ty) => { ($x, ()) };
}

#[macro_export]
macro_rules! hval {
    ($x:expr, $($tail:tt)+) => { ($x, hval!($($tail)+)) };
    ($x:expr) => { ($x, ()) };
}

#[macro_export]
macro_rules! hpat {
    ($x:pat, $($tail:tt)+) => { ($x, hpat!($($tail)+)) };
    ($x:pat) => { ($x, ()) };
}

#[macro_export]
macro_rules! with_swapped {
    ($name:ident => ($($values:expr),+); $run:expr) => {{
        let mut $name = (with_swapped!(@defaults $($values),+));
        with_swapped!(@swapped $name, $($values),+);
        let result = $run;
        with_swapped!(@swapped $name, $($values),+);
        result
    }};

    (@swapped $path:expr, $x:expr, $($tail:tt)+) => {
        std::mem::swap(&mut $path.0, &mut $x);
        with_swapped!(@swapped $path.1, $($tail)+);
    };

    (@swapped $path:expr, $x:expr) => {
        std::mem::swap(&mut $path.0, &mut $x);
    };

    (@defaults $x:expr, $($tail:tt)+) => {
        (Default::default(), with_swapped!(@defaults $($tail)+))
    };

    (@defaults $x:expr) => { (Default::default(), ()) };
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        fn a(hpat!(a, b): &mut hlist!(Vec<String>, Vec<String>)) {
            a.push("aoeu".to_string());
            b.push("stnh".to_string());
        }

        fn b(hpat!(a, b, c): &mut hlist!(Vec<String>, i64, i8)) {
            a.push("lcrg".to_string());
            *b = 10000;
            *c = 125;
        }

        #[derive(Default, Debug, PartialEq)]
        struct Scene {
            data1: Vec<String>,
            data2: Vec<String>,
            data3: i64,
            data4: i8,
        }

        let mut scene = Scene::default();

        with_swapped!(x => (scene.data1, scene.data2); a(&mut x));
        with_swapped!(x => (scene.data1, scene.data3, scene.data4); b(&mut x));

        assert_eq!(
            scene,
            Scene {
                data1: vec!["aoeu".to_owned(), "lcrg".to_owned()],
                data2: vec!["stnh".to_owned()],
                data3: 10000,
                data4: 125,
            }
        );
    }
}
