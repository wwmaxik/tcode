use nucleo::{pattern::{Pattern, CaseMatching, Normalization}, Matcher, Utf32String};

fn main() {
    let mut matcher = Matcher::new(nucleo::Config::DEFAULT);
    let pattern = Pattern::parse("main", CaseMatching::Ignore, Normalization::Smart);
    let mut scratch = Vec::new();
    let s = nucleo::Utf32Str::new("src/main.rs", &mut scratch);
    let score = pattern.score(s, &mut matcher);
    println!("{:?}", score);
}
