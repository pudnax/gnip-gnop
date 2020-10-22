#[macro_export]
macro_rules! include_str_from_outdir {
    ($t: literal) => {
        include_str!(concat!(env!("OUT_DIR"), $t))
    };
}

#[macro_export]
macro_rules! include_bytes_from_outdir {
    ($t: literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), $t))
    };
}

#[macro_export]
macro_rules! tuple_as {
    ($e:expr, ( $T0:ty, $T1:ty, $T2:ty, $T3:ty, $T4:ty, $T5:ty ) ) => {
        (
            $e.0 as $T0,
            $e.1 as $T1,
            $e.2 as $T2,
            $e.3 as $T3,
            $e.4 as $T4,
            $e.5 as $T5,
        )
    };
    ($e:expr, ( $T0:ty, $T1:ty, $T2:ty, $T3:ty, $T4:ty ) ) => {
        (
            $e.0 as $T0,
            $e.1 as $T1,
            $e.2 as $T2,
            $e.3 as $T3,
            $e.4 as $T4,
        )
    };
    ($e:expr, ( $T0:ty, $T1:ty, $T2:ty, $T3:ty ) ) => {
        ($e.0 as $T0, $e.1 as $T1, $e.2 as $T2, $e.3 as $T3)
    };
    ($e:expr, ( $T0:ty, $T1:ty, $T2:ty ) ) => {
        ($e.0 as $T0, $e.1 as $T1, $e.2 as $T2)
    };
    ($e:expr, ( $T0:ty, $T1:ty ) ) => {
        ($e.0 as $T0, $e.1 as $T1)
    };
    ($e:expr, ( $T0:ty, ) ) => {
        ($e.0 as $T0,)
    };
}
