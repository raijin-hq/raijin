use unicode_width::UnicodeWidthChar;

fn main() {
    let checkmark = '✅';
    println!("✅ width: {:?}", checkmark.width());
    
    let woman = '👩';
    println!("👩 width: {:?}", woman.width());
    
    let woman_scientist = "👩‍🔬";
    println!("👩‍🔬 total width: {}", woman_scientist.chars().map(|c| c.width().unwrap_or(0)).sum::<usize>());
}
