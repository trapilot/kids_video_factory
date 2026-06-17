macro_rules! trace {
    ($state:expr, $($arg:tt)*) => {
        println!(
            "🧭 [{} | {:?} | retry:{}] {}",
            $state.target_age,
            $state.current_node,
            $state.meta.retry_count,
            format!($($arg)*)
        );
    };
}