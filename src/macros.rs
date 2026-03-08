#[macro_export]
macro_rules! ui_println {
    ($($arg:tt)*) => ({
        // stderr: human-readable messages (tables, prompts, errors)
        $crate::output::ui_println(format_args!($($arg)*));
    })
}

#[macro_export]
macro_rules! out_println {
    ($($arg:tt)*) => ({
        // stdout: machine-readable output (TSV/JSON/magic lines)
        println!($($arg)*);
    })
}
