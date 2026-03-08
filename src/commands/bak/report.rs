use console::Style;

pub(crate) fn report(label: &str, value: &str, style: &Style) {
    let dim = Style::new().dim();
    eprint!("{}", dim.apply_to(format!("{label}: ").as_str()));
    eprintln!("{}", style.apply_to(value));
}
